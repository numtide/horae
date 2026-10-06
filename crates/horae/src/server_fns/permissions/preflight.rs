//! Read-only legacy-source diagnostics; never authority to activate a new policy.

use sqlx::PgPool;
use uuid::Uuid;

/// Independent counts may overlap; zero findings do not establish readiness.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PreflightCounts {
    pub cross_org_memberships: i64,
    pub cross_org_approvals: i64,
    pub incomplete_approval_attribution: i64,
    pub unexpected_approval_attribution: i64,
    pub unexpected_approval_states: i64,
    pub pending_imports_without_requester: i64,
    pub terminal_imports_without_requester: i64,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PreflightError {
    #[error("Permission preflight unavailable")]
    Forbidden,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Inspects legacy source data using authenticated tenant/requester IDs.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    requester: Uuid,
) -> Result<PreflightCounts, PreflightError> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("SET LOCAL idle_in_transaction_session_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id = $1 FOR SHARE",
        org_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if policy != Some(0) {
        return Err(PreflightError::Forbidden);
    }
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
        org_id,
        requester
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(PreflightError::Forbidden)?;
    let authorized = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE org_id = $1 AND id = $2 AND active AND org_role = 'admin')",
        org_id, requester
    ).fetch_one(&mut *tx).await?;
    if authorized != Some(true) {
        return Err(PreflightError::Forbidden);
    }
    // One statement snapshot; none of these counts is an activation precondition token.
    let counts = sqlx::query_as!(PreflightCounts,
        "SELECT
          (SELECT count(*) FROM assignments a
           JOIN projects p ON p.id = a.project_id JOIN users u ON u.id = a.user_id
           WHERE p.org_id <> u.org_id AND (p.org_id = $1 OR u.org_id = $1)) AS \"cross_org_memberships!\",
          (SELECT count(*) FROM approvals a JOIN users u ON u.id = a.user_id
           LEFT JOIN users approver ON approver.id = a.approved_by
           WHERE (a.org_id <> u.org_id OR a.org_id <> approver.org_id)
             AND (a.org_id = $1 OR u.org_id = $1 OR approver.org_id = $1)) AS \"cross_org_approvals!\",
          (SELECT count(*) FROM approvals WHERE org_id = $1 AND state = 'approved'
             AND (approved_by IS NULL OR approved_at IS NULL)) AS \"incomplete_approval_attribution!\",
          (SELECT count(*) FROM approvals WHERE org_id = $1 AND state <> 'approved'
             AND (approved_by IS NOT NULL OR approved_at IS NOT NULL)) AS \"unexpected_approval_attribution!\",
          (SELECT count(*) FROM approvals WHERE org_id = $1 AND state IN ('open', 'invoiced')) AS \"unexpected_approval_states!\",
          (SELECT count(*) FROM horae_jobs WHERE org_id = $1
             AND kind IN ('harvest_api_import', 'harvest_csv_import')
             AND original_requester_id IS NULL AND status IN ('queued', 'running')) AS \"pending_imports_without_requester!\",
          (SELECT count(*) FROM horae_jobs WHERE org_id = $1
             AND kind IN ('harvest_api_import', 'harvest_csv_import')
             AND original_requester_id IS NULL AND status IN ('succeeded', 'failed', 'cancelled')) AS \"terminal_imports_without_requester!\"",
        org_id
    ).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(counts)
}
