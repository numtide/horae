use crate::models::permission_editor::PermissionRequester;
use crate::reports::limits::time::{authorize_current, authorize_rows};
use horae_core::permissions::catalog::PermissionSelection;
use horae_core::types::OrgRole;

use super::*;

pub(super) enum Purpose {
    Manager,
    Projects,
    Time(i32),
}

pub(super) struct Authority {
    pub org_id: Uuid,
    pub actor_id: Uuid,
    pub purpose: Purpose,
}

impl Authority {
    pub async fn time(
        connection: &mut PgConnection,
        org_id: Uuid,
        actor_id: Uuid,
    ) -> Result<(Self, i32), StatusCode> {
        configure(connection).await?;
        sqlx::query!("SAVEPOINT csv_authority")
            .execute(&mut *connection)
            .await
            .map_err(database_error)?;
        let (policy, _) = authorize_current(
            connection,
            PermissionRequester {
                org_id,
                user_id: actor_id,
            },
        )
        .await?;
        release_authority(connection).await?;
        Ok((
            Self {
                org_id,
                actor_id,
                purpose: Purpose::Time(policy),
            },
            policy,
        ))
    }

    pub async fn begin(&self, connection: &mut PgConnection) -> Result<(), StatusCode> {
        configure(connection).await?;
        self.check(connection, &[], &[], &[]).await
    }

    pub async fn check(
        &self,
        connection: &mut PgConnection,
        project_ids: &[Uuid],
        monetary_project_ids: &[Uuid],
        contexts: &[(Uuid, Uuid)],
    ) -> Result<(), StatusCode> {
        if matches!(self.purpose, Purpose::Time(_)) {
            let grants = self.hold_time(connection).await?;
            authorize_rows(
                connection,
                PermissionRequester {
                    org_id: self.org_id,
                    user_id: self.actor_id,
                },
                &grants,
                contexts,
            )
            .await?;
            return release_authority(connection).await;
        }
        sqlx::query!("SAVEPOINT csv_authority")
            .execute(&mut *connection)
            .await
            .map_err(database_error)?;
        if matches!(self.purpose, Purpose::Projects) {
            crate::reports::limits::authorize_project_rows(
                connection,
                self.org_id,
                self.actor_id,
                project_ids,
                monetary_project_ids,
            )
            .await?;
            return release_authority(connection).await;
        }
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
        if matches!(self.purpose, Purpose::Manager)
            && !matches!(role, OrgRole::Manager | OrgRole::Admin)
        {
            return Err(StatusCode::FORBIDDEN);
        }
        release_authority(connection).await
    }

    /// Keep the same current authority across every fragment of one group.
    pub async fn hold_time(
        &self,
        connection: &mut PgConnection,
    ) -> Result<PermissionSelection, StatusCode> {
        let Purpose::Time(expected_policy) = self.purpose else {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        };
        sqlx::query!("SAVEPOINT csv_authority")
            .execute(&mut *connection)
            .await
            .map_err(database_error)?;
        let (policy, grants) = authorize_current(
            connection,
            PermissionRequester {
                org_id: self.org_id,
                user_id: self.actor_id,
            },
        )
        .await?;
        if policy != expected_policy {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(grants)
    }
}

async fn configure(connection: &mut PgConnection) -> Result<(), StatusCode> {
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *connection)
        .await
        .map_err(database_error)?;
    configure_deadlines(connection).await
}

pub(super) async fn release_authority(connection: &mut PgConnection) -> Result<(), StatusCode> {
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

pub(super) struct CsvBuffer {
    pub writer: csv::Writer<Vec<u8>>,
    project_ids: Vec<Uuid>,
    monetary_project_ids: Vec<Uuid>,
    contexts: Vec<(Uuid, Uuid)>,
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
            monetary_project_ids: Vec::new(),
            contexts: Vec::new(),
            records: 0,
            first: true,
        })
    }

    pub fn fetch_limit(&self) -> i32 {
        if self.first { 1 } else { CHUNK_ROWS as i32 }
    }

    pub async fn time_record(
        &mut self,
        sender: &mpsc::Sender<Vec<u8>>,
        connection: &mut PgConnection,
        authority: &Authority,
        context: (Uuid, Uuid),
    ) -> Result<(), StatusCode> {
        self.contexts.push(context);
        self.record(sender, connection, authority, None).await
    }

    pub async fn project_record(
        &mut self,
        sender: &mpsc::Sender<Vec<u8>>,
        connection: &mut PgConnection,
        authority: &Authority,
        row: &crate::reports::ProjectExportRow,
    ) -> Result<(), StatusCode> {
        if row.has_monetary_budget() {
            self.monetary_project_ids.push(row.id);
        }
        self.record(sender, connection, authority, Some(row.id))
            .await
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
        self.monetary_project_ids.sort_unstable();
        self.monetary_project_ids.dedup();
        self.contexts.sort_unstable();
        self.contexts.dedup();
        // Backpressure must not retain authority locks or reuse a pre-wait check.
        let permit = sender
            .reserve()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        authority
            .check(
                connection,
                &self.project_ids,
                &self.monetary_project_ids,
                &self.contexts,
            )
            .await?;
        permit.send(bytes);
        self.project_ids.clear();
        self.monetary_project_ids.clear();
        self.contexts.clear();
        self.records = 0;
        self.first = false;
        Ok(())
    }
}
