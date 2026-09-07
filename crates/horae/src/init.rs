//! `horae init` — bootstrap a fresh installation.
//!
//! Creates exactly one organization and one admin user. Nothing else: no
//! clients, no projects, no tasks, no time entries. Demo content belongs to
//! `horae seed`, which is for trying the app out, not for standing one up.

use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

/// Create the organization and its first admin user.
///
/// Refuses when an organization already exists. The alternative — a silent
/// no-op — would leave an operator who mistyped the org name believing the
/// name they passed is the one in the database.
pub async fn run(
    pool: &PgPool,
    org_name: &str,
    admin_email: &str,
    admin_name: &str,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    let existing = sqlx::query_scalar!("SELECT name FROM organizations LIMIT 1")
        .fetch_optional(&mut *tx)
        .await?;
    if let Some(name) = existing {
        anyhow::bail!(
            "Organization \"{name}\" already exists — `init` only bootstraps an empty \
             installation. Use `user create` to add further users."
        );
    }

    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, $2)",
        org_id,
        org_name,
    )
    .execute(&mut *tx)
    .await?;

    let user_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        org_id,
        admin_email,
        admin_name,
        OrgRole::Admin as OrgRole,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    println!("Created organization: {org_name} ({org_id})");
    println!("Created admin user: {admin_name} <{admin_email}> ({user_id})");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    use sqlx::PgPool;

    #[sqlx::test(migrations = "./migrations")]
    async fn init_creates_one_org_and_one_admin_and_nothing_else(pool: PgPool) {
        run(&pool, "Contoso", "ops@contoso.example", "Ops")
            .await
            .unwrap();

        let org = sqlx::query!("SELECT id, name FROM organizations")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(org.len(), 1);
        assert_eq!(org[0].name, "Contoso");

        let users = sqlx::query!(
            r#"SELECT org_id, email, name, org_role as "role: horae_core::types::OrgRole", active
               FROM users"#
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].org_id, org[0].id);
        assert_eq!(users[0].email, "ops@contoso.example");
        assert_eq!(users[0].name, "Ops");
        assert_eq!(users[0].role, horae_core::types::OrgRole::Admin);
        assert!(users[0].active);

        // No demo content came along for the ride.
        for table in ["clients", "projects", "tasks", "time_entries", "invoices"] {
            let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(count, 0, "{table} should be empty after init");
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn init_refuses_when_an_organization_exists(pool: PgPool) {
        run(&pool, "Contoso", "ops@contoso.example", "Ops")
            .await
            .unwrap();

        let err = run(&pool, "Second", "other@contoso.example", "Other")
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("already exists"), "unexpected error: {err}");

        // The refusal left nothing behind.
        let orgs: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM organizations")
            .fetch_one(&pool)
            .await
            .unwrap()
            .unwrap_or(0);
        let users: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap()
            .unwrap_or(0);
        assert_eq!((orgs, users), (1, 1));
    }
}
