use horae_core::types::OrgRole;

use super::*;

pub(super) enum Purpose {
    Manager,
    Projects,
}

pub(super) struct Authority {
    pub org_id: Uuid,
    pub actor_id: Uuid,
    pub purpose: Purpose,
}

impl Authority {
    pub async fn begin(&self, connection: &mut PgConnection) -> Result<(), StatusCode> {
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
            .execute(&mut *connection)
            .await
            .map_err(database_error)?;
        configure_deadlines(connection).await?;
        self.check(connection, &[]).await
    }

    pub async fn check(
        &self,
        connection: &mut PgConnection,
        project_ids: &[Uuid],
    ) -> Result<(), StatusCode> {
        sqlx::query!("SAVEPOINT csv_authority")
            .execute(&mut *connection)
            .await
            .map_err(database_error)?;
        sqlx::query_scalar!(
            "SELECT id FROM organizations WHERE id=$1 FOR SHARE",
            self.org_id
        )
        .fetch_optional(&mut *connection)
        .await
        .map_err(database_error)?
        .ok_or(StatusCode::FORBIDDEN)?;
        let role = sqlx::query_scalar!(
            r#"SELECT org_role AS "role: OrgRole" FROM users WHERE id=$1 AND org_id=$2 AND active FOR SHARE"#,
            self.actor_id, self.org_id,
        ).fetch_optional(&mut *connection).await.map_err(database_error)?
            .ok_or(StatusCode::FORBIDDEN)?;
        match self.purpose {
            Purpose::Manager if !matches!(role, OrgRole::Manager | OrgRole::Admin) => {
                return Err(StatusCode::FORBIDDEN);
            }
            Purpose::Projects => {
                crate::reports::limits::authorize_project_rows(
                    connection,
                    self.org_id,
                    self.actor_id,
                    project_ids,
                )
                .await?
            }
            Purpose::Manager => {}
        }
        // Rollback releases row locks without destroying the outer source cursor.
        // RELEASE also removes the savepoint frame on arbitrarily long exports.
        sqlx::query!("ROLLBACK TO SAVEPOINT csv_authority")
            .execute(&mut *connection)
            .await
            .map_err(database_error)?;
        sqlx::query!("RELEASE SAVEPOINT csv_authority")
            .execute(connection)
            .await
            .map_err(database_error)?;
        Ok(())
    }
}

pub(super) struct CsvBuffer {
    pub writer: csv::Writer<Vec<u8>>,
    project_ids: Vec<Uuid>,
    records: usize,
    first: bool,
}

impl CsvBuffer {
    pub fn new(headers: &[&str]) -> Result<Self, StatusCode> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer
            .write_record(headers)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Self {
            writer,
            project_ids: Vec::new(),
            records: 0,
            first: true,
        })
    }

    pub fn fetch_limit(&self) -> i32 {
        if self.first { 1 } else { CHUNK_ROWS as i32 }
    }

    pub async fn record(
        &mut self,
        sender: &mpsc::Sender<Vec<u8>>,
        connection: &mut PgConnection,
        authority: &Authority,
        project_id: Option<Uuid>,
    ) -> Result<(), StatusCode> {
        if let Some(id) = project_id {
            self.project_ids.push(id);
        }
        self.records += 1;
        self.writer
            .flush()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if self.first || self.records >= CHUNK_ROWS || self.writer.get_ref().len() >= CHUNK_BYTES {
            self.flush(sender, connection, authority).await?;
        }
        Ok(())
    }

    pub async fn flush(
        &mut self,
        sender: &mpsc::Sender<Vec<u8>>,
        connection: &mut PgConnection,
        authority: &Authority,
    ) -> Result<(), StatusCode> {
        self.writer
            .flush()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if self.writer.get_ref().is_empty() {
            return Ok(());
        }
        let writer = std::mem::replace(&mut self.writer, csv::Writer::from_writer(Vec::new()));
        let bytes = writer
            .into_inner()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        self.project_ids.sort_unstable();
        self.project_ids.dedup();
        // Backpressure must not retain authority locks or reuse a pre-wait check.
        let permit = sender
            .reserve()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        authority.check(connection, &self.project_ids).await?;
        permit.send(bytes);
        self.project_ids.clear();
        self.records = 0;
        self.first = false;
        Ok(())
    }
}
