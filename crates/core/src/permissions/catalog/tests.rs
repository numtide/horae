use std::collections::BTreeSet;

use serde::{Deserialize, de::value::StrDeserializer};

use super::{BuiltInProfile, Permission, PermissionEditError, PermissionSelection};
use Permission::*;

fn set(permissions: &[Permission]) -> BTreeSet<Permission> {
    permissions.iter().copied().collect()
}

#[test]
fn member_has_only_own_time_and_expense_access() {
    assert_eq!(
        BuiltInProfile::Member
            .selection()
            .iter()
            .collect::<BTreeSet<_>>(),
        set(&[TimeReadOwn, TimeWriteOwn, ExpenseReadOwn, ExpenseWriteOwn]),
    );
}

#[test]
fn catalog_has_fifty_distinct_known_grants() {
    assert_eq!(Permission::ALL.len(), 50);
    assert_eq!(set(Permission::ALL).len(), 50);
}

#[test]
fn empty_selection_includes_member_floor() {
    assert_eq!(
        PermissionSelection::new(&[]),
        BuiltInProfile::Member.selection()
    );
}

#[test]
fn floor_cannot_be_removed_or_partially_modified() {
    for permission in [TimeReadOwn, TimeWriteOwn, ExpenseReadOwn, ExpenseWriteOwn] {
        let mut selection = BuiltInProfile::Administrator.selection();
        let before = selection.clone();
        assert_eq!(
            selection.remove(permission),
            Err(PermissionEditError::MemberFloor)
        );
        assert_eq!(selection, before);
    }
}

#[test]
fn approve_all_includes_read_scopes_but_not_write_or_expense_management() {
    let selection = PermissionSelection::new(&[TimeApproveAll]);
    assert_eq!(
        selection.iter().collect::<BTreeSet<_>>(),
        set(&[
            TimeApproveAll,
            TimeApproveManaged,
            TimeReadAll,
            TimeReadManaged,
            TimeReadOwn,
            TimeWriteOwn,
            ExpenseReadOwn,
            ExpenseWriteOwn,
        ])
    );
}

#[test]
fn report_and_withdrawal_grants_do_not_add_unrelated_authority() {
    let requested = [
        ReportProfitabilityRead,
        ReportContractorRead,
        ReportInvoicingRead,
        ApprovalWithdrawManaged,
    ];
    let mut expected = set(&[TimeReadOwn, TimeWriteOwn, ExpenseReadOwn, ExpenseWriteOwn]);
    expected.extend(requested);
    assert_eq!(
        PermissionSelection::new(&requested)
            .iter()
            .collect::<BTreeSet<_>>(),
        expected
    );
}

#[test]
fn removing_managed_time_read_also_removes_all_reads_writes_and_approvals() {
    let mut selection = PermissionSelection::new(&[TimeWriteAll, TimeApproveAll]);
    selection.remove(TimeReadManaged).unwrap();
    assert_eq!(selection, BuiltInProfile::Member.selection());
}

#[test]
fn removing_invoice_draft_write_keeps_read_but_removes_managed_and_all_write() {
    let mut selection = PermissionSelection::new(&[InvoiceWriteAll]);
    selection.remove(InvoiceDraftWriteManaged).unwrap();
    assert_eq!(selection, PermissionSelection::new(&[InvoiceReadAll]));
}

#[test]
fn unknown_permission_names_are_rejected() {
    for name in [
        "administrator",
        "59",
        "60",
        "time_write_everything",
        "",
        "forecast_read_all",
    ] {
        assert!(
            Permission::deserialize(StrDeserializer::<serde::de::value::Error>::new(name)).is_err(),
            "{name}"
        );
    }
}

#[test]
fn known_permission_name_decodes_without_legacy_role_conversion() {
    assert_eq!(
        Permission::deserialize(StrDeserializer::<serde::de::value::Error>::new(
            "time_read_own"
        ))
        .unwrap(),
        TimeReadOwn,
    );
}

#[test]
fn normalization_is_idempotent_order_independent_and_duplicate_free() {
    for &a in Permission::ALL {
        for &b in Permission::ALL {
            let selection = PermissionSelection::new(&[a, b, a]);
            assert_eq!(selection, PermissionSelection::new(&[b, a]));
            assert_eq!(
                selection,
                PermissionSelection::new(&selection.iter().collect::<Vec<_>>())
            );
        }
    }
}

#[test]
fn removing_any_non_floor_grant_keeps_the_remaining_selection_closed() {
    for &permission in Permission::ALL {
        let mut selection = BuiltInProfile::Administrator.selection();
        if selection.remove(permission).is_ok() {
            assert!(!selection.contains(permission));
            assert_eq!(
                selection,
                PermissionSelection::new(&selection.iter().collect::<Vec<_>>())
            );
        }
    }
}

#[test]
fn adding_all_catalog_grants_produces_exactly_the_catalog() {
    let mut selection = PermissionSelection::new(&[]);
    for &permission in Permission::ALL {
        selection.add(permission);
    }
    assert_eq!(
        selection.iter().collect::<BTreeSet<_>>(),
        set(Permission::ALL)
    );
}

#[test]
fn dependency_graph_matches_the_reference_edges_without_extra_cross_resource_grants() {
    let edges: &[(Permission, &[Permission])] = &[
        (TimeWriteOwn, &[TimeReadOwn]),
        (TimeReadManaged, &[TimeReadOwn]),
        (TimeWriteManaged, &[TimeReadManaged, TimeWriteOwn]),
        (TimeApproveManaged, &[TimeReadManaged]),
        (TimeReadAll, &[TimeReadManaged]),
        (TimeWriteAll, &[TimeReadAll, TimeWriteManaged]),
        (TimeApproveAll, &[TimeReadAll, TimeApproveManaged]),
        (ExpenseWriteOwn, &[ExpenseReadOwn]),
        (ExpenseReadManaged, &[ExpenseReadOwn]),
        (ExpenseWriteManaged, &[ExpenseReadManaged, ExpenseWriteOwn]),
        (ExpenseReadAll, &[ExpenseReadManaged]),
        (ExpenseWriteAll, &[ExpenseReadAll, ExpenseWriteManaged]),
        (ProjectWriteManaged, &[ProjectReadManaged]),
        (ProjectCreateAll, &[ProjectReadAll]),
        (ProjectReadAll, &[ProjectReadManaged]),
        (ProjectWriteAll, &[ProjectReadAll, ProjectWriteManaged]),
        (ClientWriteAll, &[ClientReadAll]),
        (TaskWriteAll, &[TaskReadAll]),
        (PeopleWriteManaged, &[PeopleReadManaged]),
        (PeopleReadAll, &[PeopleReadManaged]),
        (PeopleWriteAll, &[PeopleReadAll, PeopleWriteManaged]),
        (BillableRateWriteManaged, &[BillableRateReadManaged]),
        (BillableRateReadAll, &[BillableRateReadManaged]),
        (
            BillableRateWriteAll,
            &[BillableRateReadAll, BillableRateWriteManaged],
        ),
        (CostRateWriteAll, &[CostRateReadAll]),
        (InvoiceDraftWriteManaged, &[InvoiceReadManaged]),
        (
            InvoiceWriteManaged,
            &[InvoiceReadManaged, InvoiceDraftWriteManaged],
        ),
        (InvoiceReadAll, &[InvoiceReadManaged]),
        (InvoiceWriteAll, &[InvoiceReadAll, InvoiceWriteManaged]),
        (EstimateWriteAll, &[EstimateReadAll]),
        (SavedReportWriteInactive, &[SavedReportReadInactive]),
        (CompanyWrite, &[CompanyRead]),
        (BillingWrite, &[BillingRead]),
    ];
    for &permission in Permission::ALL {
        let expected = edges
            .iter()
            .find(|(candidate, _)| *candidate == permission)
            .map_or(&[][..], |(_, required)| *required);
        assert_eq!(
            set(permission.prerequisites()),
            set(expected),
            "{permission:?}"
        );
    }
}

#[test]
fn removing_absent_permission_is_an_idempotent_no_op() {
    let mut selection = BuiltInProfile::ProjectManager.selection();
    let before = selection.clone();
    selection.remove(CostRateWriteAll).unwrap();
    selection.remove(CostRateWriteAll).unwrap();
    assert_eq!(selection, before);
}

#[test]
fn removing_one_financial_permission_preserves_unrelated_families() {
    let mut selection =
        PermissionSelection::new(&[BillableRateWriteAll, CostRateWriteAll, InvoiceWriteAll]);
    selection.remove(BillableRateReadManaged).unwrap();
    assert_eq!(
        selection,
        PermissionSelection::new(&[CostRateWriteAll, InvoiceWriteAll])
    );
}

#[test]
fn every_profile_includes_floor_and_is_closed_under_dependencies() {
    for &profile in BuiltInProfile::ALL {
        let selection = profile.selection();
        for permission in [TimeReadOwn, TimeWriteOwn, ExpenseReadOwn, ExpenseWriteOwn] {
            assert!(
                selection.contains(permission),
                "{profile:?}: {permission:?}"
            );
        }
        for permission in selection.iter() {
            for &required in permission.prerequisites() {
                assert!(
                    selection.contains(required),
                    "{profile:?}: {permission:?} requires {required:?}"
                );
            }
        }
    }
}

#[test]
fn profiles_reject_legacy_rank_names_and_unknown_values() {
    for name in ["admin", "manager", "owner", "", "custom"] {
        assert!(
            BuiltInProfile::deserialize(StrDeserializer::<serde::de::value::Error>::new(name))
                .is_err(),
            "{name}"
        );
    }
    for (name, profile) in [
        ("member", BuiltInProfile::Member),
        ("project_manager", BuiltInProfile::ProjectManager),
        ("people_admin", BuiltInProfile::PeopleAdmin),
        ("accounting", BuiltInProfile::Accounting),
        ("executive_manager", BuiltInProfile::ExecutiveManager),
        ("administrator", BuiltInProfile::Administrator),
    ] {
        assert_eq!(
            BuiltInProfile::deserialize(StrDeserializer::<serde::de::value::Error>::new(name))
                .unwrap(),
            profile
        );
    }
}

#[test]
fn direct_profile_defaults_match_the_known_reference_catalog() {
    // Explicit fixture sets, independent of the dependency implementation.
    let cases: &[(BuiltInProfile, &[Permission])] = &[
        (
            BuiltInProfile::Member,
            &[ExpenseReadOwn, ExpenseWriteOwn, TimeWriteOwn, TimeReadOwn],
        ),
        (
            BuiltInProfile::ProjectManager,
            &[
                ExpenseReadManaged,
                ExpenseWriteManaged,
                ProjectCreateAll,
                ExpenseReadOwn,
                ExpenseWriteOwn,
                ClientReadAll,
                ClientWriteAll,
                TimeReadManaged,
                TimeWriteOwn,
                ProjectReadManaged,
                ProjectWriteManaged,
                TimeReadOwn,
                TaskReadAll,
                TaskWriteAll,
                TimeWriteManaged,
                TimeApproveManaged,
            ],
        ),
        (
            BuiltInProfile::PeopleAdmin,
            &[
                ExpenseReadAll,
                ExpenseWriteAll,
                PeopleReadAll,
                PeopleWriteAll,
                TimeReadAll,
                TimeWriteAll,
                ProjectReadAll,
                ApprovalWithdrawManaged,
                ReportContractorRead,
                TimeApproveAll,
            ],
        ),
        (
            BuiltInProfile::Accounting,
            &[
                ExpenseReadAll,
                ExpenseWriteAll,
                BillableRateReadAll,
                CostRateReadAll,
                TimeReadAll,
                ClientReadAll,
                ClientWriteAll,
                ProjectReadAll,
                InvoiceReadAll,
                InvoiceWriteAll,
                EstimateWriteAll,
                EstimateReadAll,
                SavedReportWriteInactive,
                SavedReportReadInactive,
                ReportProfitabilityRead,
                ReportInvoicingRead,
            ],
        ),
        (
            BuiltInProfile::ExecutiveManager,
            &[
                ExpenseReadAll,
                ExpenseWriteAll,
                PeopleReadAll,
                PeopleWriteAll,
                BillableRateReadAll,
                CostRateReadAll,
                ProjectCreateAll,
                TimeReadAll,
                TimeWriteAll,
                ClientReadAll,
                ClientWriteAll,
                ProjectReadAll,
                ProjectWriteAll,
                InvoiceReadAll,
                InvoiceWriteAll,
                EstimateWriteAll,
                TaskReadAll,
                TaskWriteAll,
                EstimateReadAll,
                SavedReportWriteInactive,
                SavedReportReadInactive,
                ApprovalWithdrawManaged,
                ReportProfitabilityRead,
                ReportContractorRead,
                ReportInvoicingRead,
                TimeApproveAll,
            ],
        ),
        (
            BuiltInProfile::Administrator,
            &[
                ExpenseReadAll,
                ExpenseWriteAll,
                PeopleReadAll,
                PeopleWriteAll,
                CompanyRead,
                CompanyWrite,
                BillableRateReadAll,
                BillableRateWriteAll,
                CostRateReadAll,
                CostRateWriteAll,
                ProjectCreateAll,
                TimeReadAll,
                TimeWriteAll,
                BillingRead,
                BillingWrite,
                ClientReadAll,
                ClientWriteAll,
                ProjectReadAll,
                ProjectWriteAll,
                InvoiceReadAll,
                InvoiceWriteAll,
                EstimateWriteAll,
                TaskReadAll,
                TaskWriteAll,
                EstimateReadAll,
                SavedReportWriteInactive,
                SavedReportReadInactive,
                ApprovalWithdrawManaged,
                ReportProfitabilityRead,
                ReportContractorRead,
                ReportInvoicingRead,
                TimeApproveAll,
            ],
        ),
    ];
    assert_eq!(cases.len(), BuiltInProfile::ALL.len());
    for &(profile, expected) in cases {
        assert_eq!(
            set(profile.direct_permissions()),
            set(expected),
            "{profile:?}"
        );
        assert_eq!(
            profile.direct_permissions().len(),
            expected.len(),
            "{profile:?}"
        );
    }
}

#[test]
fn administrator_selection_covers_known_catalog_without_unknown_ids() {
    assert_eq!(
        BuiltInProfile::Administrator
            .selection()
            .iter()
            .collect::<BTreeSet<_>>(),
        set(Permission::ALL)
    );
}

#[test]
fn finance_profiles_cannot_edit_rates_and_project_manager_has_no_financial_grants() {
    for profile in [BuiltInProfile::Accounting, BuiltInProfile::ExecutiveManager] {
        let selection = profile.selection();
        assert!(selection.contains(BillableRateReadAll));
        assert!(selection.contains(CostRateReadAll));
        for permission in [
            BillableRateWriteAll,
            BillableRateWriteManaged,
            CostRateWriteAll,
        ] {
            assert!(
                !selection.contains(permission),
                "{profile:?}: {permission:?}"
            );
        }
    }
    let selection = BuiltInProfile::ProjectManager.selection();
    for permission in [
        BillableRateReadAll,
        BillableRateReadManaged,
        CostRateReadAll,
        InvoiceReadAll,
        InvoiceReadManaged,
        ApprovalWithdrawManaged,
    ] {
        assert!(!selection.contains(permission), "{permission:?}");
    }
}

#[test]
fn every_declared_dependency_is_added_and_removing_it_removes_the_dependent() {
    for &permission in Permission::ALL {
        let selection = PermissionSelection::new(&[permission]);
        for &required in permission.prerequisites() {
            assert!(
                selection.contains(required),
                "{permission:?} requires {required:?}"
            );
            let mut edited = selection.clone();
            if edited.remove(required).is_ok() {
                assert!(
                    !edited.contains(permission),
                    "{required:?} removed, {permission:?} retained"
                );
            }
        }
    }
}
