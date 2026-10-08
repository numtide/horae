# Permission PR separation

## Active delivery phase — 2026-10-08

The current goal supersedes the earlier no-merge phase: reduce open PRs by
integrating verified work through the protected merge queue and closing only
superseded deliveries whose content is preserved. Do not add new features.
Keep #212 open and unchanged as the reference, including its original branch,
worktree and unpublished changes, as explicitly requested. Historical
no-merge statements below describe the previous phase, not current authority.

Revalidated #282 at e39f033a against master8b3cc257: required Flake Check/Format
and Nixbot317 passed, the diff remains two test/configuration files, and no
review blocker is recorded. After source-boundary review, marked it ready and
submitted it without bypass to the protected queue. Entry MQE_lQDOTRPZ888AAAABHMRHd84AA_LZzgMoeSs
is first, AWAITING_CHECKS. Merge-group run37750157434 is live at
c9ea1f39964784115ca52b7c41af32ad94ff73c7. Submission is not a completed merge.

The existing #286 head ed286e6b already contains all #283–#286 corrections and
passed native and both-Linux Nixbot413 gates. Deliver these together through
#286 instead of four sequential PRs: eight files,105 additions/27 deletions
beyond #282, with just one production header addition. Preserve every commit
and downstream head. Required master/merge-group checks still apply. Close
#283/#284/#285 only after their content is confirmed integrated; preserve their
branches. #212 must not be merged or closed. Next: confirm #282's queue result
and retarget the unchanged #286 delivery to master with its complete scope.

Collected run37750157434: Flake Check and Format both succeeded. #282 merged
through the queue at08:30:34 UTC as c9ea1f39964784115ca52b7c41af32ad94ff73c7.
Retargeted #286 to master, updated its title/body to describe the complete
consolidated scope, and marked it ready. Its head remains ed286e6b; no branch
rewrite or new code. Auto-merge was enabled at08:32:18 UTC and awaits required
master-targeted checks before queue entry. #283/#284/#285 remain open until
integration is confirmed. Live read-back verifies #212 still OPEN/draft at
db3935db364f2a8aa193f0e938ce40ecc01a2f92. Next: collect #286's required checks
and queue result, then confirm the integrated tree before closing duplicates.

GitHub did not start its required pull-request workflow on base retarget or
ready-for-review: it listens to the default opened/synchronize/reopened events.
Closed and immediately reopened only #286 to trigger that workflow, verified
the unchanged ed286e6b head/master base, and re-enabled protected auto-merge.
No empty commit, branch rewrite or protection bypass was used. Run37750804732
is now live on ed286e6b: Format passed, Flake Check remains in progress. Poll
that existing run at60-second intervals; do not rerun it on silence. #282's remote merge tree matches e39f033a's
tree exactly (2493f371dc87b6f7bcd57efce0a3319aa9669358).

Direct Nixbot page verification corrects the earlier build489 attribution:
build413 succeeded on ed286e6b (16 succeeded, four already built); build489
was cancelled by the close/reopen transition, not successful. Its cancelled
attributes appear as failures in GitHub. Build490 is the replacement for the
same head and is actively building, with15 of20 attributes successful at08:42
UTC. GitHub's aggregate check still exposes an older cancellation timestamp,
so consult the actual build page as well as the PR rollup. Disabled auto-merge
temporarily until the replacement build and required GitHub checks finish.
No source change, retry, or additional build was requested. Next: collect
build490 and run37750804732; restore protected auto-merge only after acceptance.

Readiness audit after that correction: all57 code extractions have terminal
Nixbot failures at their published heads, not still-running checks. #220 at
ea78c27c remains the first functional candidate after shared CI, but build431
failed ARM tests during compilation, browser interaction, and OIDC VM startup.
The OIDC log shows the guest booting slowly and hitting the900-second driver
connection deadline, before authentication assertions. These observations do
not prove a source defect or OOM. Re-read the production snapshot helper,
its three callers and all seven reader regressions: no new source blocker
identified in that boundary. Do not merge it on native-only historical results
or start a mass retry/rebase. #212 remains excluded from delivery.

GitHub run37750804732 completed successfully at08:45:57 UTC: Flake Check
and Format both passed on ed286e6b. The existing watcher exited successfully.
Nixbot490 remains live with17 of20 attributes successful; ARM browser,
deployment and OIDC VM tests are still running. Its live browser log contains
passing cases and the OIDC guest is booting; neither is assumed stuck.
Auto-merge remains disabled until this build finishes. All three heads from
#283–#285 are ancestors of ed286e6b; final integrated-tree verification is still
required before closing those PRs.

Nixbot490 subsequently completed successfully: all20 attributes succeeded on
the unchanged ed286e6b head. GitHub's required checks also passed. Submit the
verified delivery to the protected squash queue; do not bypass its merge-group
checks. Expected integrated tree:1a3b16230479c34c2e7b0363e711ae5fd4630514.

Queue submission confirmed: #286 is first, AWAITING_CHECKS, head unchanged.
Merge-group run37754261233 is queued on f29e72b46dd7c023c4366c127a3a1d81fc087689
against master c9ea1f39964784115ca52b7c41af32ad94ff73c7. Follow this run rather
than the completed PR run. No merge is claimed yet and #283–#285 stay open.

Merge-group run37754261233 passed both required checks and #286 merged as
f29e72b46dd7c023c4366c127a3a1d81fc087689. The actual tree is
4a72442027ece979333e42c5262efc3ab1b21290, not the expected ed286e6b tree:
the squash integration retained the17-line local wait_for_session_release
helper in csv_streaming.rs from #282 alongside the shared helper in its parent.
Direct two-tree comparison finds no other difference and no lost delivery
content. Both helpers have the same lock/release/Busy handling; this is test
duplication, not a runtime change. The queue's checks cover the integrated tree.
An attempted auto-merge disable failed because the PR had already merged;
no bypass, rollback, or branch rewrite was performed. Reconcile the retained
helper when integrating existing import-cleanup #224, not in a new PR.
Close #283–#285 as superseded while preserving their branches. #212 remains open.

Read-back confirms #283/#284/#285 CLOSED, not merged separately; #283 was
already closed when the explicit close command ran. #212 remains OPEN at
db3935db364f2a8aa193f0e938ce40ecc01a2f92. Source branches were not deleted.

Prepared only the next functional delivery, #220. Its clean isolated worktree
was rebased from ed286e6b onto master f29e72b4 with rebase.updateRefs disabled.
Backup ref refs/backup/pr220-before-master-20261008 preserves ea78c27c.
New head fa9eeaaa964cd1963ddde16e9e503795df5fdfd8 was published with an exact
old-head force-with-lease; no dependent branch was rewritten. Range-diff shows
the repair commit unchanged. The only whole-tree delta from its previous head
is the17-line helper inherited from the master squash integration. Updated
the PR body with current hashes and explicit historical-versus-current gates.
Next: verify fresh #220 checks, deliver through the protected queue when green,
then verify integration before closing overlapping #217. Do not merge #212.

Started a bounded second independent delivery while #220's fresh CI runs:
#219 now has head552341232c273eac3cd1dea33c223a669b7f91fb on master f29e72b4.
Its clean worktree was rebased with updateRefs disabled; backup ref
refs/backup/pr219-before-master-20261008 retains690cce20. Range-diff proves the
single pure-domain extraction patch unchanged; only the existing master
helper differs in the full tree. Published with an exact old-head lease and
updated the existing PR description. No other feature branch was rewritten.
Keep delivery concurrency to #219/#220 until these checks resolve. #220's
GitHub run37755060825 is active; Format has passed. Both PRs
remain drafts pending fresh acceptance, with no auto-merge requested yet.

Direct build-page verification finds stale Nixbot detail links in the fresh
check rollups: build493 still names old #220 head ea78c27c, and build492 names
old #219 head690cce20. Both old builds are running and already have failed
attributes; they must not be counted as acceptance for fa9eeaaa/55234123.
The repository build list currently ends at503 and does not yet show either
new SHA. Do not cancel/restart these builds solely because the links are stale.
GitHub's current #219 run is37755399793 and #220 is37755060825; their SHA
associations are correct. Verify Nixbot's actual commit before using results.

#220 GitHub run37755060825 completed successfully on fa9eeaaa: Flake Check
10m50s and Format37s. Its watcher exited normally. #219 run37755399793 remains
live. Nixbot's fresh evaluation check runs remain in progress with links to
old builds492/493; no new-head Nixbot acceptance is established yet. Keep the
two deliveries out of the merge queue until those current-head gates resolve.

#219 GitHub run37755399793 also passed (Flake Check11m6s, Format37s); both
GitHub watchers have exited successfully. Its Nixbot evaluation check then
completed and reported failure through build492, still labelled690cce20 rather
than current55234123. Build492's ARM test compiler received SIGKILL and the
package failed, preventing browser/deployment/OIDC dependencies from running.
No OOM diagnosis or current-tree acceptance follows from those logs.
Requested a single reevaluation of current-head check113238559934 through
GitHub's rerequest endpoint; it returned404 and did not start a run. Next
fallback is one close/reopen of #219 without changing its head, then verify
the build's actual SHA. Do not loop retries or interrupt live #220 work.

That single close/reopen completed. Read-back confirms #219 OPEN/draft at
unchanged55234123. GitHub started run37757142397; the earlier run remains a
pass for the same SHA. Nixbot has not yet exposed a replacement build in the
repository list (still ending at503). #220's evaluation check remains live.
Next: follow those exact runs and verify any Nixbot replacement's source SHA.

The reopen run37757142397 finished successfully (Flake Check47s, Format37s),
but Nixbot still reports the terminal492 failure on current55234123 and has
not created a replacement build. The documented rerequest endpoint is the
POST path already attempted (GitHub REST Checks documentation); no malformed
method explains the404. No browser/MCP tools are loaded in this session.
Asked the user asynchronously for Nixbot operator assistance to verify the
old-build/current-SHA association and run the actual current commit. Do not
repeat close/reopen or change source to work around unverified CI provenance.
#220's evaluation check remains live and its linked old build493 still runs
three attributes, with two earlier failed attributes. This is not acceptance
of fa9eeaaa and not evidence that the live work has stopped. Continue watching
the existing evaluation while keeping both PRs out of the merge queue.

Blocked audit: the current-SHA/old-build attribution problem has persisted
through three consecutive goal turns, including the single reopen fallback,
the operator request, and fresh API/build-page checks. #219's check is terminal
failure on the current SHA but still refers to old-head build492. #220's
current evaluation remains live while its referenced old-head build493 already
has failed attributes; finishing that old build does not provide current-head
acceptance. All code extraction gates in the last full inventory were failed;
starting additional branches would not resolve this verification problem.

The documented Nixbot restart/cancel interface requires an authenticated
Nixbot login (repo writers and PR authors qualify); anonymous access here is
read-only. The GitHub rerequest attempt returned404, and no browser/MCP session
is available. No safe verified merge remains until an operator or authorized
Nixbot session resolves the attribution and evaluates current55234123 and
fa9eeaaa. Mark the goal blocked, not complete; do not cancel still-running
builds. Completed delivery stays #282/#286 merged and #283–#285 closed. Keep
#212 OPEN and unchanged. Resume by inspecting actual head/build associations,
then follow protected-queue delivery and close #217 only after #220 integrates.

Source: https://github.com/Mic92/nixbot#authentication-backend (restart/login
roles); deployed version and the cause of the attribution discrepancy remain
unverified. No claim that the completed native checks imply ARM acceptance.

### Authenticated CI provenance correction — 2026-10-08 13:41 UTC

The user opened Nixbot in Chrome. The existing Playwright client now has an
authenticated session and exposes the per-attribute restart controls; no API
token or cookie was extracted. The dedicated Nixbot tab leaves other tabs alone.

The preceding attribution-blocker diagnosis was incorrect. Read-only Nixbot
API responses expose the actual stored tree hashes:

- Build492 / #219: c58b6c2b711102eaa8b29897322815a2bc3a066c, exactly the tree
  of current55234123 and of merging old690cce20 into master f29e72b4.
- Build493 / #220: c6f34c82f8db7dcc5326538acd7cc8a5689ebd17, exactly the tree
  of currentfa9eeaaa and of merging oldea78c27c into master f29e72b4.

Verified with git merge-tree --write-tree and git rev-parse; neither operation
changes branches or worktree files. Upstream orchestrator.py merges the PR
into its base before identifying a build by tree hash; build_reuse.py replays
the result onto other commits with identical trees. Thus the older displayed
commit is not a provenance failure: these are failing checks of the current
content. No new commit, reopen, or reevaluation is needed to fix attribution.

Both builds are now terminal failures. #219 has five failed/dependency-failed
ARM attributes; #220 has three failed ARM attributes (browser, sqlx-prepare,
tests). Current-head GitHub Flake Check and Format remain successful. Browser
inspection of #220 sqlx-prepare confirms rustc received signal9/SIGKILL;
this does not prove OOM or identify who terminated it. Separate browser
failures still need their own diagnosis. No merge acceptance is claimed.

The repository list now also shows successful builds500–503 for #239, #240,
#242 and #248. The earlier all-failed inventory is stale; these are candidates
for fresh head/tree and scope verification, not automatic merge approvals.

Requested one bounded diagnostic retry through the inspected per-attribute
restart form for #220's ARM sqlx-prepare. API read-back confirms it is building
since13:40:48 UTC on the same stored tree. Browser and tests retain their
earlier failed results; no other attribute was restarted. Next: inspect this
retry's result before any further retry. Do not restart the whole matrix or
weaken checks. Keep #212 unchanged.

Sources: Nixbot /api/repos/github/numtide/horae/builds/492 and /493;
https://github.com/Mic92/nixbot/blob/main/nixbot/nixbot/orchestrator.py;
https://github.com/Mic92/nixbot/blob/main/nixbot/nixbot/build_reuse.py.

### Protected delivery resumed — 2026-10-08

The user requested continuation. Build493's isolated ARM sqlx-prepare retry
succeeded at13:42:57 UTC without source changes. Requested one isolated retry
of its ARM tests through the authenticated per-attribute API (HTTP200), not a
full-matrix restart. Browser's original failure is separately identified:
project-bulk-recovery.cjs expects two .proj-row elements at line49 but observes
zero within5 seconds; slow SQL statements accompany it, without proving cause.

Revalidated independent #239 at53633d76 against master f29e72b4. Nixbot500
succeeded on both architectures and its stored tree45e0c85f exactly matches
the proposed merge. Renewed bounded source review traced session-derived
manager identity, approve/reopen paths, organization filters, entry state
preservation, event totals and four regression tests; no new blocker within
the tenant-isolation repair. No new permission policy is activated.

Marked #239 ready and closed/reopened it once, unchanged, to activate the
master-only GitHub workflow after automatic base retargeting. Updated its
description with current evidence. Run37788133547 passed Format38s and Flake
Check57s. Protected auto-merge entered position1, AWAITING_CHECKS, with
merge-group run37788371582 at27b27ad5f1f58a50eaf04cf219647d8437b11e57.
Next: collect that queue result and verify the integrated tree before claiming
merge. #212 remains untouched. No source edits or new PRs were made.

#242 and #240 also have successful Nixbot502/501 builds. #242's renewed review
checks shared-model serialization, internal SQLx invoice relation, separate
Harvest/plugin payloads, frontend consumers and preserved HTTP/unit assertions.
It remains a candidate, not queued; revalidate its integration base after #239.

### Approval isolation integrated; next bounded delivery — 2026-10-08

#239 merged at13:57:50 UTC as27b27ad5f1f58a50eaf04cf219647d8437b11e57.
Merge-group run37788371582 passed Flake Check59s and Format47s. The actual
integrated tree45e0c85fc943f8fb817d7391175cf33f368fb7c4 exactly equals the
Nixbot500 tree and the predicted merge; master read-back confirms the commit.
Updated the PR description with the completed integration evidence.

Prepared #242 at unchanged4c173f0a: renewed review found no blocker in the
serialization boundary, SQLx preservation or separate UI/plugin/Harvest
consumers. Marked ready and closed/reopened once to activate required GitHub
checks. Current base27b27ad5 produces expected tree
8aca91cd4562dfa70be63b6662bd997d39ba1a13; its net delta is exactly six payload
and test/cache files,173 additions, preserving all of #239 and the shared CI
helper. Updated #242's description; no auto-merge is enabled yet.

GitHub run37788740850 is the new workflow. Nixbot now links build504; inspect
its actual stored tree and terminal attributes before trusting the rollup,
which temporarily retains timestamps/conclusions from earlier build502.
#220's ARM test retry remains live after the successful SQLx retry. Its
browser failure was traced to initial fixture loading in project-bulk-recovery,
before any mocked mutation; no timeout or assertion was changed.

Next: collect #242's current combined-tree acceptance and submit to the
protected queue only when green. Inspect the isolated #220 test result before
any further retry. #212 remains unchanged; no new feature or PR was created.

### Identity delivery review and isolated CI recovery — 2026-10-08

Previous goal iteration was progress: #239 merged with exact-tree verification.
Current read-back confirms #242 still open at4c173f0a and both required GitHub
checks passed in run37788740850. Nixbot504's actual stored tree is the expected
8aca91cd4562dfa70be63b6662bd997d39ba1a13 and is building without failures;
the older green conclusion in GitHub must not substitute for its live result.

#220's isolated ARM test retry succeeded without source changes. Requested one
browser-attribute retry through the authenticated API after confirming that
success (HTTP200). Build493 now has15 successes, four cached attributes and
one browser attribute building. This is a verified wait, not a blocker or an
acceptance claim. Its tree predates #239 and needs integration-base revalidation
before delivery even if the final retry passes.

Completed renewed #240 source review at unchangedce053157: exact session and
directory response shapes; preserved activity/role and tenant guards; forged
targets; archived and renamed approval labels; unchanged internal financial
projections; frontend consumers; three HTTP matrices and production-page
rendering/action tests. No critical/high finding in the selected repair.
Trial merge with master27b27ad5 is clean at treeba906f4a and keeps #239's
approve/reopen isolation. This trial is not test acceptance.

Trial combination with #242 identifies exactly one textual conflict in the
shared authorization_tests.rs fixture registration. Both PRs add independent
modules/calls there; preserve all four matrices when resolving it. No worktree
or branch was changed by merge-tree. #240 remains draft and will be prepared
after #242 lands, avoiding another matrix on a soon-obsolete base. Updated
its PR description with the current review and precise remaining steps.

Next: collect live Nixbot504 and493, deliver #242 when its exact integration
tree passes, then resolve the identified #240 test-registration conflict in
its existing worktree. Preserve #212 and all original work.

### Verified waits and documentation provenance — 2026-10-08

Revalidated master27b27ad5 and live Nixbot493/504. #220's browser retry has
passed the original initial-project-list failure point and progressed through
invoice recovery assertions, but remains building. #242's current integration
has12 successful and four cached attributes; only its two browsers and ARM
deployment/OIDC tests remain live. No failure, restart or acceptance inferred
from elapsed time. These are verified waits on actual build records.

Independently reviewed #248 preservation at a6d2e091. Exactly54 feature015
documents exist;48 match originaldb3935db byte-for-byte. Read the complete
six-file delta: only extraction/historical-evidence context and clarification
that constitution1.1.0 remains an unadopted proposal. No requirement or task
state changed. AGENTS matches the original; the three New Project permission
transition hunks retain pre-cutover boundaries. The review patch changes56
Markdown paths only; git diff --check passes. This is provenance review, not
fresh Spec Kit completion or renewed Harvest research. Keep #248 draft until
its current integration gates are verified; no extra build was triggered.

Next remains #242 after Nixbot504 succeeds, followed by #240's known fixture
registration reconciliation. Preserve the running #220 retry and revalidate
its new merge base before delivery. No source changes or additional merges
occurred in this iteration; #212 remains untouched.

Nixbot493 subsequently finished successfully:16 attributes passed and four
were cached. The three isolated ARM retries all passed without source or
assertion changes; original failures remain evidence, not a diagnosed root
cause. Updated #220's description to distinguish this verified treec6f34c82
from the still-unverified combination with updated master. Keep it draft and
do not close #217 yet. Nixbot504 remains live with13 passed, four cached and
three final checks building. Read-back confirms #212 remains OPEN/draft at
unchangeddb3935db364f2a8aa193f0e938ce40ecc01a2f92.

### Time-entry response delivery and identity integration — 2026-10-08

Nixbot504 completed successfully, including the ARM deployment recovery test,
on tree8aca91cd4562dfa70be63b6662bd997d39ba1a13. Revalidated #242 head4c173f0a
and master27b27ad5, then submitted to the protected queue. Merge-group
run37793044838 passed and #242 merged at14:31:57 UTC as
40434acda95755b8f332b3d7f15a036fe724e644. The remote commit tree exactly matches
the Nixbot-tested tree. Updated the existing PR description; no bypass or
branch deletion. #212 remains outside delivery.

Prepared #240 in its existing clean worktree after that merge. Backup ref
refs/backup/pr240-before-master-20261008 preserves ce053157. Rebased its four
commits onto40434acd with rebase.updateRefs disabled and unsigned commits.
Resolved the only conflict in authorization_tests.rs by retaining registration
and execution of all four matrices: time_entry_payload, session_identity,
user_directory and approval_labels. Range-diff shows only that context
reconciliation; the other three patches are unchanged. The whole-tree delta
from the previous head consists of already-merged master changes. #239's
approval mutations remain intact; #240 only changes approval read labels.

New head4ef9a3c8f9cdf6116a548c36bed287806d05f4af, tree
acb5d8123615803b46f8e67656681b4076441973. git diff --check and cargo fmt --check
inside nix develop pass. Previous Nixbot501 is historical, not acceptance of
the combined tree. Keep #240 draft until fresh CI succeeds. No new feature,
PR, dependent-branch rewrite or test relaxation.

Publication confirmed with an exact old-head force-with-lease. The initial
HTTPS push waited for a graphical credential prompt and was interrupted;
the successful push used the existing GitHub CLI credential helper only for
that invocation, without changing global configuration. GitHub run37793715132
and Nixbot505 are live. The latter's stored tree exactly matches acb5d812 above,
with no failures at this check. Remote #212 remains OPEN/draft at db3935db.

Next: collect these fresh GitHub and Nixbot checks, then protected-queue
delivery and exact-tree verification. #220
still needs current-base verification; #217 stays open until its replacement
actually integrates. Keep #212 and its unpublished work untouched.

### Follow-on integration review during #240 CI — 2026-10-08

Nixbot505 remains live on exact treeacb5d812, with no failed attributes;
GitHub37793715132 Format passed while Flake Check remains running. This is a
verified wait. No retry, new build or branch change was requested this iteration.

Renewed #224 review at af4656d7 covered release_import, every production caller,
worker joins and the three cleanup regressions. Read the pinned local SQLx0.8.6
flush/wait_until_ready/start_rollback implementation: the loop consumes queued
responses, recovers only3B001, and does not enqueue retries of mutations. Explicit
ROLLBACK precedes unlock and close; close-on-drop protects error exits. No new
critical/high source finding in this boundary. Its patch is still nine files;
all seven query descriptors and three regression tests are retained. Current
combined runtime acceptance remains required.

Read-only trial merge of #224 with #240 is clean at tree1a57ea03. The inherited
17-line local CSV wait_for_session_release duplicates the parent's helper;
remove that local definition during #224 integration, retaining the existing
wildcard import, calls, deadlines and assertions. Do not create another PR.

Trial #220/#240 combination identifies one test-registration conflict, not a
production conflict: retain legacy_readers plus all four existing matrices and
their calls. Its improved exact route matching merges independently. No branch
or worktree was changed by either trial. Next remains #240 after current checks
pass, then #220's preserved replacement of #217; #224 review is ready for its
subsequent integration. #212 remains excluded.

### Bounded two-PR delivery pipeline — 2026-10-08

#240/Nixbot505 progressed to its last three ARM checks (browser, deployment,
OIDC), without failures. Rather than prepare #220 against a soon-obsolete base,
stacked that existing PR on the exact #240 head4ef9a3c8. This preserves a bounded
two-delivery pipeline and a reviewable #220 diff; it does not add features or PRs.

Preserved fa9eeaaa at refs/backup/pr220-before-identity-20261008, then rebased its
single repair commit in the existing worktree with updateRefs and signing off.
The only conflict was fixture registration; all five test matrices and calls
remain. Range-diff confirms unchanged production and regression-test patches;
the identical query07580408 descriptor is now inherited from #240. Explicit
old/new comparisons confirm its blob and the snapshot/reader tests unchanged.
An initial read-only comparison accidentally used the root worktree's old HEAD;
discarded that output and repeated with explicit fa9eeaaa/6afef017 revisions.

Published6afef017ac15cef7d902a967c08a700bb4376c9e with the exact previous-head
lease; no dependent branch was rewritten. Updated #220's base to
fix/identity-response-projections and documented the dependency. Its own delta
is26 files,1119 additions/17 deletions; remains draft without auto-merge.
Nix-shell cargo fmt and git diff --check pass. GitHub37795259871 and Nixbot506
are live. Nixbot's stored treedf84fa671f9db4175e3a7d2dc4e97beebccbf8a6 exactly
matches the head and its merge with current master40434acd. No failed attribute
at this check; previous Nixbot493 is explicitly historical.

Next: deliver #240 first after505 and GitHub37793715132 pass. Verify its actual
merge tree, then retarget/reconcile #220 onto that master without dropping any
patch and verify the same combined tree before trusting506. Only after #220
passes current-base and protected-queue checks may #217 be closed as replaced.
Do not merge the stack out of order; keep #212 unchanged. No build was retried.

### ARM VM failure and isolated recovery — 2026-10-08

GitHub37793715132 passed #240 Flake Check (19m44s) and Format (42s); its watcher
exited0. Nixbot505 then failed both ARM VM checks while its other18 attributes
passed or were cached. Both failures are the initial900-second wait for the
guest's connecting-to-host driver signal, before functional assertions. Logs
show QEMU cannot initialize KVM and the guest still progressing through boot;
this does not identify the root cause or prove an application defect.

Inspected both failing logs and the matching checked-in test scripts. Requested
one isolated e2e retry through the authenticated browser API; HTTP200 confirmed
acceptance and the public API now shows that same attribute building. The slow
browser response was observed, not treated as failure or duplicated. No source,
timeout, assertion or infrastructure setting changed. OIDC remains failed and
has not been retried. Updated #240's PR description with the outstanding gates.
Nixbot506 for #220 remains live with its three ARM browser/VM checks running.

Next: collect the existing e2e retry and506. If the isolated #240 e2e passes,
request one OIDC-attribute retry, not a full-matrix restart. If the same failure
recurs, preserve its evidence and investigate before another retry. No merge
until the exact combined tree has full acceptance. #212 remains untouched.

GitHub37795259871 subsequently completed successfully for #220 at6afef017.
Nixbot506's ARM browser also passed; its deployment VM has connected to the
driver and passed the health check, now exercising database backup. Both VM
checks remain running, not accepted yet. #240's single deployment retry is
also live; its OIDC attribute remains failed and unretried. These are verified
waits, with no new source change or restart in this iteration. Continue the
same handles and do not equate both green GitHub workflows with full acceptance.

### Reader acceptance and final identity gate — 2026-10-08

Nixbot506 completed successfully for #220:16 passed and four cached attributes,
including both architectures' browsers, deployment recovery and OIDC. Its
stored treedf84fa671f9db4175e3a7d2dc4e97beebccbf8a6 matches the combined head;
no attribute retry was needed for506. GitHub37795259871 also passed. Updated
the PR body. Keep #220 draft until its parent #240 integrates, then verify the
actual master combination; never merge this PR into the feature-branch base.

#240's single ARM deployment retry subsequently passed its full functional
scenario. Nixbot505 then had only OIDC failed. Requested exactly one OIDC retry
after checking that deployment was succeeded; authenticated API returned200,
and public read-back confirms only checks.aarch64-linux.e2e-oidc is building.
No code, timeout, assertion or infrastructure change. Both original startup
timeouts remain recorded; retries do not establish a root-cause fix.

Next: collect the live OIDC retry. If505 is fully successful, revalidate #240
head4ef9a3c8/current master40434acd and treeacb5d812, then mark ready and submit
to the protected queue. Verify its final tree before reconciling #220 and
using506's acceptance. Close #217 only after #220 integrates. Preserve #212.

### Local import cleanup preparation — 2026-10-08

Preserved #224's published af4656d7 at
refs/backup/pr224-before-reader-20261008. Rebasing its single correction onto
#220 head6afef017 completed without conflicts as072d0c35; range-diff confirms
the original patch is identical. Added local unsigned commitd9edb456, removing
only the17-line duplicate CSV test helper. Both existing CSV callers now use
the identical parent helper through their existing super import. Assertions,
deadlines, SQL descriptors and production behavior were not changed by that
cleanup. `nix develop --command cargo fmt --all -- --check` and
`git diff --check` passed. No database or full-suite execution is claimed for
this combined tree385dcca94f7008093c422b931db2bfbe5c5d45bc.

This preparation remains local, with no push or additional remote CI. #224's
published head remains af4656d7. After #240 and #220 integrate, rebase these
two commits onto actual master, verify the resulting tree and patch range,
then publish using an exact-head lease and require combined CI acceptance.
At15:38 UTC,505's isolated OIDC retry is still building; its log shows the
guest progressing through PostgreSQL initialization. #220 remains fully green
but draft on its feature-branch base. Next: collect505, then integrate #240
and #220 in that order. Preserve #212 and keep #217 until #220 is verified merged.

### Local domain foundation preparation — 2026-10-08

While the existing505 OIDC retry runs, prepared #219 locally after #224.
Preserved published55234123 at
refs/backup/pr219-before-import-cleanup-20261008, then rebased only its single
commit from f29e72b4 onto local #224 headd9edb456 with updateRefs disabled.
New local head8cef61a060e9743c5b56c095cd9ef91a26890f41 has tree
117adb9bb1583481a417d944265770217f90cab6. The rebase had no conflicts;
range-diff reports an identical patch and comparing crates/core against the
published head produces no difference. No source or test edits were needed.

`nix develop --command cargo test -p horae-core` passed all158 tests, with no
failures or ignored tests. Core Clippy with all targets and `-D warnings` also
passed. `git diff --check` passed and the worktree is clean.
These domain tests do not certify server integration or the full Nix matrix.
No push or extra remote build was started; #219 remains published at55234123.
After the preceding deliveries merge, reconcile the single owned commit onto
actual master, preserve this patch, and require combined CI before merging.

Nixbot505 remains building as of15:44 UTC. The read-only watcher session62108
observes that same build every60 seconds and does not retry builds. Next:
collect its terminal result, then integrate #240 and #220 if accepted; keep
#217 open until #220 is verified integrated and leave #212 unchanged.

### Identity CI accepted and queued — 2026-10-08

Nixbot505 finished successfully for exact treeacb5d812:16 attributes passed
and four were cached. The single isolated ARM OIDC retry completed its full
test script in819 seconds, including authenticated API access and rejection
after account deactivation. The original startup failures remain recorded;
no code, timeout or assertion changed for either isolated VM retry. The
read-only watcher session62108 completed normally and must not be restarted.

Revalidated #240 OPEN/draft at4ef9a3c8, master40434acd, a clean worktree and
an exact head tree matching505. GitHub37793715132 already passed. Updated the
PR body with acceptance, marked ready and requested protected auto-merge with
an exact-head guard. Read-back confirms queue entry
MQE_lQDOTRPZ888AAAABG-OeHs4AA_LZzgMqRxY at position1, stateQUEUED. This is not
yet a completed merge. Next: collect its merge-group checks and verify final
treeacb5d812, then reconcile #220 onto actual master and require the existing
df84fa67 acceptance to match exactly. Close #217 only after #220 integrates.
Local #224/#219 preparations remain unpublished; #212 remains untouched.

### Identity merged; reader delivery reconciled — 2026-10-08

Merge-group37803792651 passed and #240 merged at15:49:56 UTC as
1998388f1040b161b88bca144dccb4bc22491e87. GitHub confirms MERGED and master
at that commit. Its remote tree is exactlyacb5d812, the accepted505 tree.
The merge-group watcher59999 exited successfully. No protection bypass or
branch deletion was used.

Retargeted #220 to master and rebased only its single owned commit from4ef9a3c8
onto actual master1998388f. Backup
refs/backup/pr220-before-identity-merge-20261008 preserves6afef017. New head
91244589414b05cbe6d8ee85a5dc04227a247db8 has exactly the same complete tree
df84fa671f9db4175e3a7d2dc4e97beebccbf8a6 already accepted by Nixbot506.
No conflict; range-diff reports the same patch, and a whole-tree diff is empty.
Publication uses an exact old-head lease. Current-head GitHub and protected
merge-group checks remain required before final integration. Next: verify the
published head and check read-back, queue #220 when accepted, then verify its
actual merge tree before closing #217. #212 remains unchanged; #224/#219 stay
local until their preceding deliveries integrate.

Published #220 read-back confirms OPEN/draft, master base and91244589 head.
Nixbot has already replayed506's successful evaluation/build onto this exact
head because its tree is unchanged; no matrix restart was requested. GitHub
run37804193972 is live on91244589 for Flake Check and Format. Watcher11465
follows that run every60 seconds; collect it rather than starting another run.
Both #240's final merge evidence and #220's refreshed delivery body are published.

### Reader merged, duplicate closed, import cleanup published — 2026-10-08

GitHub37804193972 passed for #220 at91244589. Marked ready and submitted to
the protected queue with an exact-head guard; merge-group37804431884 passed.
#220 merged at15:54:41 UTC as7b85d2c3c85472d579f312b1b4149c16747fef3d.
GitHub confirms MERGED, master at that commit, and the remote tree exactly
df84fa67, matching Nixbot506. Updated its published delivery receipt.

After this verification, closed #217 as superseded by #220. Read-back confirms
CLOSED at unchangeddd141c5c; its branch was not deleted. #212 remains
OPEN/draft atdb3935db and its worktree/unpublished changes were not touched.

Rebased #224's two prepared commits onto actual master7b85d2c3, producing
be84b332ec74c8dac7943691288109720dbe0083. Both patches are identical in
range-diff and the whole tree still equals the prepared385dcca9 tree. The
review delta is10 files,268 additions/19 deletions, including the17-line
duplicate-helper removal. Published with an exact af4656d7 lease; no new PR
or branch was created. Fresh combined-tree CI is required before merge.
Next: collect #224 current-head GitHub/Nixbot results and review any failures
before retrying. #219 remains local at8cef61a0 with158 core tests and Clippy
passed; reconcile it after #224 integrates, without rewriting other branches.

#224 read-back confirms master base andbe84b332 head. GitHub37804884447 is
live (Format passed); watcher66999 follows it every60 seconds. Nixbot509 is
building exact tree385dcca9, with no failed attributes at the initial check.
Use those existing handles; do not substitute historical447 or restart on silence.

### Documentation delivery prepared locally — 2026-10-08

While #224's existing CI runs, preserved #248 publisheda6d2e091 at
refs/backup/pr248-before-delivery-chain-20261008. Rebased its three owned
documentation commits fromed286e6b onto local #219 head8cef61a0, with dependent
ref updates disabled. The rebase had no conflicts; range-diff marks all three
patches identical. New local headf8f029fd85fe831d42b0919e22ef5d54efc3ea82 has
treee3a656f4b276149826c76ac160b0c68393d9c540.

All56 owned Markdown files compare byte-for-byte with the published #248 head.
The broader specs-directory comparison additionally shows only the inherited
legacy-reader-authority.md from #220; that file exactly matches the new base
and is not a #248 edit. No requirements, product decisions, task checkboxes or
constitution adoption changed. `nix fmt -- --ci` passed:493 files processed,
zero changed. `git diff --check` passed. This is integration/provenance work,
not a fresh Spec Kit completion or a server-test acceptance claim.

No #248 push or extra remote build was started. Its published head remains
a6d2e091. After #224 and #219 integrate, reconcile only these three documentation
commits onto actual master and require current integration gates. #224's
GitHub37804884447 and Nixbot509 remain live;509 has eight successful and four
cached attributes with no failures at this check. Next: collect those existing
handles, merge #224 only after acceptance, then continue #219 and #248.

### Delivery index clarified; same import CI still running — 2026-10-08

Marked the obsolete pre-merge handoff explicitly historical and updated the
delivery-index introduction to reflect the six merged PRs and four superseded
closures. Kept the dependency rows for traceability rather than presenting
completed entries as pending deliveries. A fresh GitHub read confirms #212,
#219, #224, #248 and #218 remain at their expected published heads.

Nixbot509 remains live at16:08 UTC with16 of20 attributes successful/cached.
Only ARM tests, browser, deployment and OIDC remain building; no failures are
recorded. Read-only watcher22948 now follows that exact build every60 seconds;
it cannot restart it. GitHub watcher66999 still follows37804884447. Next:
collect those handles, then revalidate #224 head/master/tree before protected
queue submission. No new source change, remote build or PR was created.

### Import regression evidence on the current tree — 2026-10-08

Nixbot509's ARM test suite passed:836 server tests, zero failures and11 existing
ignored cases. Its log explicitly confirms untracked-transaction rollback,
savepoint-release recovery and CSV cancellation/checkpoint regressions pass.
The build now has17 successful/cached attributes. Only ARM browser, deployment
and OIDC remain live; browser logs show passing mobile fee-form cases and the
OIDC guest is still booting. GitHub37804884447 remains live on Flake Check.
Updated #224's published body with the current run links and this partial
acceptance, without claiming full CI acceptance. No retries or source changes.
Next: continue watcher22948 and watcher66999; require both terminal passes
before revalidating the exact head/tree and submitting #224 to the queue.

At16:22 UTC,509's ARM browser passed, leaving only ARM deployment and OIDC
running (18 of20 attributes successful/cached). Both guest logs show Horae
listening on its application port; functional VM completion is not yet claimed.
GitHub37804884447 still runs Flake Check. This iteration is a verified wait on
the same watcher22948/66999 handles, with no restart, branch change or merge.
Next remains full CI acceptance followed by exact-tree queue verification.

### GitHub accepted; initial ARM OIDC startup failed — 2026-10-08

GitHub37804884447 completed successfully for #224: Flake Check25m34s and
Format41s. Watcher66999 exited0 and must not be polled again. Nixbot509's
ARM OIDC attribute then failed its initial driver wait at900.81 seconds,
before any login assertion. Horae had started; late virtual-console/device
initialization continued near the deadline. The failure is not evidence of an
OIDC application regression, nor proof of its infrastructure root cause.
Both Nix VM definitions and flake.lock are byte-identical to integrated master.

The deployment VM did connect before its deadline and is actively exercising
functional tests, including database privilege repair. Do not restart or cancel
that live attribute. Chrome client7643 remains connected to Nixbot; a read-only
authenticated API call confirmed509's exact385dcca9 tree, failed OIDC and
building deployment. That browser call finished; no browser request is pending.
Published #224's body now reflects the successful GitHub run and OIDC failure.

Next: collect deployment through watcher22948. After deployment finishes,
revalidate its result and request exactly one isolated OIDC-attribute retry,
without source, timeout or assertion changes. No retry has yet been requested
for509. If that retry repeats the failure, retain evidence and investigate
before another attempt. Keep #224 draft and unmerged until full acceptance;
#219/#248 remain locally prepared and unpublished, and #212 stays unchanged.

### Deployment accepted; single OIDC retry running — 2026-10-08

Nixbot509's ARM deployment check passed its complete1776-second functional
script, including both interrupted-import recovery scenarios and final row/report
assertions. The initial build is terminal failed only because of OIDC startup;
watcher22948 exited normally and must not be polled again. Public API read-back
confirmed the deployment success, the unchanged385dcca9 tree, and only OIDC failed.

After those checks, requested exactly one restart of
checks.aarch64-linux.e2e-oidc through the authenticated Chrome session7643.
The request revalidated the tree, failed OIDC state and all other attributes'
acceptance before POST. API returned200 and public read-back confirms OIDC is
building. No full restart, source edit, timeout change or assertion relaxation.
The browser call completed; no browser request is pending. Updated #224's
published body with deployment acceptance and the isolated retry status.

Read-only watcher96352 now observes the retry every60 seconds; its initial
16:42 UTC read confirms only OIDC building. GitHub37804884447 is already green.
Next: collect96352. If successful, revalidate #224 headbe84b332, master7b85d2c3
and expected tree385dcca9 before protected queue submission and final-tree
verification. If the same retry fails, investigate before another attempt.
Do not send a second automatic retry. #219/#248 preparations and #212 remain intact.

### Import report converter reviewed and prepared locally — 2026-10-08

While509's single OIDC retry runs, renewed the bounded #223 review using the
Rust, async and minimal-change guidance. Traced the migration caller, complete
converter, chunk writer, lease checkpoint fence, FK definitions and all five
added concurrency regressions. Discovery holds no job lock; organization SHARE
precedes the tenant/job/oversize recheck under explicit READ COMMITTED. Waiting
converters preserve newly bounded worker checkpoints and claims; replacement,
deletion, competing converters and ungated worker FK checks are covered by the
retained tests. The transaction stays on one connection and rolls back partial
archive work on errors. No new critical/high finding within this repair.

Preserved published68661980 at
refs/backup/pr223-before-delivery-chain-20261008, then rebased its single commit
fromed286e6b onto local #248 headf8f029fd with updateRefs disabled. New local
head8a9efae5b60454130c02bdec74fcc9d2ba56ce3b has tree
cc17441f8aea988af103e9568b4928d9045811c7. No conflict. Both owned Rust files and
all14 added/replacement SQLx descriptors are byte-identical to68661980; the
obsolete locking-discovery descriptor remains absent. The only range-diff
change is descriptor33678a5f now inherited unchanged from the base.

Nix-shell Cargo formatting and whitespace checks passed; the worktree is clean.
No runtime test or full CI acceptance is claimed for this new composition.
Historical native acceptance of68661980 does not certify this tree. No source
edit, database operation, push or extra remote build occurred. #223 remains
published at68661980 and follows the already prepared #224/#219/#248 sequence;
these local composition bases are not new functional dependencies.

Next: collect watcher96352 for #224's one OIDC retry (still building at16:47
UTC). Do not retry it again automatically. If it passes, revalidate and queue
#224, verify its merged tree, then reconcile and deliver the prepared roots in
order. Keep #212 unchanged and do not create additional PRs.

### Import cleanup merged; permission domain published — 2026-10-08

#224 merged at 16:58:22 UTC as
`2d3e67217b75b374bb5002ffed30b9a15f7325c6`. GitHub CI 37804884447,
protected merge-group 37812776719 and Nixbot 509 passed. The integrated tree
`385dcca94f7008093c422b931db2bfbe5c5d45bc` exactly matches acceptance.
Nixbot finished with 16 passed attributes and four cached. Its one isolated ARM
OIDC retry passed the complete login/API/deactivation script in 780.91 seconds;
the initial startup-delay cause remains unproven. No further retry is needed.
The completed watchers are no longer active. The final receipt is in #224.

Reconciled #219's one owned commit onto that actual master with automatic ref
updates disabled. Backup `refs/backup/pr219-before-import-merge-20261008`
preserves `8cef61a0`. Published head
`8123201e86bbbda7445a37e2991a2d26160ad3fc` with an exact lease against
`552341232c273eac3cd1dea33c223a669b7f91fb`. Range-diff confirms the unchanged
patch and its entire tree equals the prepared tree
`117adb9bb1583481a417d944265770217f90cab6`. Local Nix-shell acceptance on that
tree remains 158 core tests and all-target core Clippy with warnings denied.
No source change, migration or permission activation occurred.

#219 remains draft pending current combined CI; its published description now
identifies the actual head, base and acceptance boundary. #248 and #223 remain
local preparations, not published or CI-certified combinations. #212 remains
the untouched reference. No additional PR was created.

Current checks started at 17:03 UTC: GitHub 37813612403 and Nixbot 510.
Nixbot's stored tree is exactly `117adb9bb1583481a417d944265770217f90cab6`;
both formatting attributes passed, eight compilation/test attributes are
building, and browser/VM checks remain pending. Read-only GitHub watcher
42207 checks every 60 seconds. Remote #212 read-back remains open/draft at
`db3935db364f2a8aa193f0e938ce40ecc01a2f92`. The ledger is committed locally;
its newer delivery receipts have not yet been pushed to #218.

Next: collect watcher 42207 and Nixbot 510, verify complete acceptance, and use
the protected queue only after acceptance. Then reconcile #248 and #223 in
order onto actual merged master; retain all source branches.

### Dependent domain rules prepared without another remote build — 2026-10-08

The preceding iteration made progress: #219's rebased head was published and
its exact-tree GitHub/Nixbot acceptance started. Current read-back confirms
GitHub 37813612403 and Nixbot 510 remain live, not stopped. Formatting passed;
the x86 SQLx, Clippy, package and OIDC checks have passed while other checks
continue. No retry or duplicate CI run was requested.

Renewed #221's bounded review against its person-management, rate-scope and
approval-visibility contracts. Read all six implementation/test files and the
shared scope/prerequisite rules. Exact original-source comparison still passes
against #212 at `db3935db364f2a8aa193f0e938ce40ecc01a2f92`. There are no runtime
callers in this delivery. Field ownership cannot cross the person/project
namespace, cost grants remain independent, explicit reset/zero writes require
write authority, and approval/read coverage must hold for every supplied record.
Complete selection acquisition, self-approval, locks and transaction authority
remain explicit consumer obligations, not claims of this pure module. No new
high/critical finding within this extraction boundary.

Saved `37b124bea7188f0ee84ad49055f44ba4d7cd920c` at
`refs/backup/pr221-before-delivery-chain-20261008`. Rebased only its owned commit
from `690cce20` onto the prepared #223 head `8a9efae5`, with automatic ref updates
disabled. Local head `c05533c1ec6a23466741880798527b30ed92c51d` has tree
`11d2953ef6f85e9ae891d9515c9ca27372f2fd92`. No conflict; range-diff is identical
and all seven owned file blobs are unchanged. This composition order does not
add a functional dependency beyond #219.

Fresh local Nix-shell checks passed: 187 core tests, zero failed/ignored;
all-target core Clippy with warnings denied; Cargo formatting and whitespace.
No source edit, new dependency, database operation, push or remote build.
#221 remains published/draft at `37b124be`; the new combination still needs
reconciliation onto actual master and full applicable CI before delivery.

Next: collect #219 watcher 42207 and Nixbot 510; only queue after all required
acceptance. Continue #248 then #223, followed by the prepared #221. #212 and
all source branches remain preserved. This ledger update is local, not yet pushed.

### Inactive storage mechanically reconciled locally — 2026-10-08

The preceding iteration made progress by reviewing and preparing #221 with
187 passing core tests and Clippy. #219's existing GitHub 37813612403 watcher
42207 and Nixbot 510 remain live. The current Nixbot read-back has no failed
attribute; ARM compilation/tests and browser/VM checks are still outstanding.
No queue submission or retry was made while acceptance is incomplete.

Confirmed #222 is still draft at published
`e8d89b469b57f49eaffab2262cc54c1b93517ac7`, with two owned commits based on
`690cce20`. Preserved it at
`refs/backup/pr222-before-delivery-chain-20261008`, then rebased those two
commits onto local #221 `c05533c1` with automatic ref updates disabled.
Local head `b0acd09fc4b927dae9b99b7febec7a90c157ee71` has tree
`0fb2e447576bd610fff8c9efd42685d132b292d1`. Both patches are identical in
range-diff and the rebase had no conflicts. #221 is a local delivery-order
predecessor, not a new functional dependency; #222 still requires only #219.

Compared all 29 owned files against the published head. All are byte-identical
except `server_fns.rs`, whose differences have exactly the same stable patch ID
as the inherited base changes from #239/#220 (tenant-bound week totals and the
snapshot module). Both migrations retain original blobs/checksums; all 19 SQLx
descriptors and the original README note are unchanged. No schema statement,
name-comparison policy, stored-grant rule or assertion was changed.

Fresh Nix-shell core tests passed: 189, zero failed/ignored. Full formatting
check processed 502 files with zero changes; whitespace checks passed. All-target
core Clippy with warnings denied passed (session 47822 completed). No database connection, migration execution,
SQLx regeneration, push or additional remote build occurred. This mechanical
preparation does not replace storage review or full combined-tree CI; prior
database results in #222 remain historical.

Next: collect #219's existing checks. Deliver #219 only after
acceptance, followed by #248/#223/#221 and then #222 after its storage-specific
verification. Keep #212 and original branches intact. Ledger remains local.

### Permission domain CI advances to final ARM checks — 2026-10-08

The preceding iteration made progress by preparing #222 without publication.
This iteration is a verified wait on GitHub 37813612403 (watcher 42207) and
Nixbot 510, both confirmed live. At approximately 17:14 UTC, every x86 Nixbot
attribute is accepted, and ARM Clippy, SQLx, formatting and package checks are
accepted. ARM tests, browser, deployment VM and OIDC VM remain building with
no recorded failure. Direct log tails show executing tests and successful
browser cases, not just a stale status flag. The stored tree remains
`117adb9bb1583481a417d944265770217f90cab6`.

No source edit, branch publication, retry or queue submission during this wait.
Next: collect those four ARM attributes and the existing GitHub run; only then
consider #219 ready for protected delivery. Other local preparations stay parked.

### Permission domain ARM tests accepted — 2026-10-08

Verified wait continued on the same live handles, not a new build. Nixbot 510's
ARM server suite now passes: 836 successful tests, 11 existing manual ignored
cases and all auxiliary binaries successful. The raw log confirms the terminal
test summaries. ARM browser/deployment/OIDC remain building, as does GitHub
Flake Check in 37813612403 (watcher 42207). No failure or retry is recorded.
#219's description now links these exact runs and distinguishes partial from
complete acceptance. Next: collect the remaining checks before queue entry.

### Inactive storage review completed on the prepared tree — 2026-10-08

While #219's same three ARM checks continue, reviewed #222 at local `b0acd09f`
against its existing storage contract/checklist. Read both complete migrations,
both loaders and native decoder, models/module wiring, name-validation changes,
strict catalog restoration and all 11 database regressions. Confirmed by source
search that no application caller consumes the new storage or activates policy.

Loaders bind tenant and record identity, distinguish absence from malformed
state, reject unknown/duplicate/incomplete/future grants without repair, and
preserve administrative identity independently of provenance. Composite FKs,
source-shape checks and restrictive template deletion retain tenant boundaries.
The Unicode-name index replacement and its rollback regressions preserve the
original storage contract; UUIDs, roles and existing assignments are not rewritten.
Consumers still own authentication, current activity, locking and activation;
none is inferred from successful loading. No new high/critical finding in this
bounded storage review. No new Harvest observation or broader parity claim.

Reviewed source/migration blobs remain identical to published `e8d89b46`; the
worktree is clean. No code, test or migration was changed or executed during
this review. Existing local 189-core-test/Clippy/format acceptance remains valid;
fresh database/full integration acceptance is still required after publication.
Next: finish #219's existing checks, then follow the recorded delivery order.

### Permission domain GitHub CI and ARM browser passed — 2026-10-08

Verified wait produced new acceptance: GitHub run 37813612403 completed
successfully (Flake Check 24m2s; Format 42s). Watcher 42207 is finished; do not
restart or poll it. Nixbot 510's ARM browser also passed. Only ARM deployment
and OIDC remain building, with no failure recorded. Both logs show Horae
listening at approximately 17:25 UTC, but that does not certify their complete
functional scripts. No retry was requested. Published #219's updated check
summary without changing its head or draft state.

Next: collect Nixbot 510's two VM results. If accepted, revalidate #219's head,
master and exact integration tree before protected queue submission. Preserve
all prepared work and #212; do not create another PR or restart completed checks.

### Permission domain VM startup failures and isolated retry — 2026-10-08

Nixbot 510's original ARM deployment and OIDC attempts both terminated in the
900-second test-driver connection wait, before functional assertions. Exact
timeouts were 900.58s and 900.61s. Logs show network/backdoor initialization
near the deadline; the underlying delay cause is not proven. All other Nixbot
attributes and GitHub 37813612403 remain accepted. No full restart is warranted.

Used the existing authenticated Chrome connection to request exactly one
isolated deployment retry. The browser-side guard verified origin, tree
`117adb9bb1583481a417d944265770217f90cab6`, both terminal VM failures and all
other accepted/cached attributes before POST. HTTP 200 confirmed; public
read-back at approximately 17:34 UTC shows deployment building and OIDC still
failed. Browser call completed and client 7643 is idle. No code, timeout or
assertion changes. OIDC has not been retried and must not be started in parallel.

Read-only watcher 85069 checks deployment every 60 seconds and exits when that
attribute becomes terminal. Its initial read confirms building; no restart logic
is included. #219's published body records the same partial acceptance boundary.

Next: collect watcher 85069; if successful, consider one isolated OIDC
retry. If deployment fails again, inspect the new evidence before any further
attempt. #219 stays draft and unmerged. Preserve the existing prepared delivery
chain and #212; do not restart GitHub or already accepted checks.

### Deployment retry reaches functional assertions — 2026-10-08

Verified wait on watcher 85069 continues without another retry. At approximately
17:48 UTC, Nixbot 510's ARM deployment log has passed the driver connection,
TCP port wait and `/health` assertion and is starting the database backup test.
This attempt therefore progressed beyond the original connection timeout.
The full deployment script is still running, so no acceptance or merge is claimed.
OIDC remains at its initial failure and has not been retried. Next: collect
85069 to completion before considering the single sequential OIDC retry.

### Deployment accepted; final OIDC retry started — 2026-10-08

Collected watcher 85069: exit 0, deployment accepted. Nixbot 510's raw log
confirms the full script finished in 1588.83 seconds, including restart recovery,
entry counts and persisted report checks. Do not poll or restart that watcher.
The aggregate build still showed failed solely because of the original OIDC
startup timeout; all other attributes were accepted or cached.

After that verification, requested exactly one isolated OIDC retry through the
existing authenticated Chrome client. Guard verified expected tree
`117adb9bb1583481a417d944265770217f90cab6`, failed OIDC and every other attribute
accepted before POST. HTTP 200 and public read-back confirm OIDC building as
the sole remaining check. Browser call completed; client 7643 is idle. No
source, timeout, assertion or complete-build restart. GitHub remains green.

Read-only watcher 43370 observes OIDC every 60 seconds and exits at its terminal
result. Its first read confirms building. It has no retry logic.

Next: collect watcher 43370. If successful, revalidate current #219 head
and master, queue via protection, and verify the final integrated tree. If it
fails, investigate before any additional retry. #219 remains draft/unmerged;
all prepared work and #212 remain untouched. Receipt is published in #219;
the separation ledger remains locally committed rather than published.

### Documentation delivery receipt prepared while OIDC runs — 2026-10-08

Revalidated #248's draft remote head `a6d2e091` and clean local preparation
`f8f029fd` (tree `e3a656f4b276149826c76ac160b0c68393d9c540`). All 56 owned
Markdown files still match the published blobs. Prepared the concise delivery
body at `.scratch/pr248-merge-delivery.md`, explicitly separating preserved
research/task history from fresh acceptance, leaving the constitution proposal
unadopted, and noting #217's separate closure after #220. This receipt is local
and must be updated with the actual rebased/published head before use. No push,
new CI, source edit, changed product decision or new review-completion claim.

The existing OIDC watcher 43370 remains live on Nixbot 510's sole pending
attribute. Next: collect that retry, deliver #219 only after complete acceptance,
then reconcile #248's three commits onto the actual merged master and publish
with its known remote-head lease. Keep #212 and the prepared chain intact.

### Permission domain fully accepted and queued — 2026-10-08

OIDC watcher 43370 exited successfully. Nixbot 510 is green: 16 passed attributes
and four cached, exact tree `117adb9bb1583481a417d944265770217f90cab6`. The
single sequential OIDC retry completed its whole script in 708.20 seconds,
including authenticated API access and rejection of a deactivated account.
GitHub 37813612403 is already accepted. Neither initial startup timeout's root
cause is established by the successful retries. Watchers 43370/85069 are done.

Revalidated #219 head `8123201e`, actual master `2d3e6721`, direct ancestry,
mergeability and absence of unresolved review threads. Published full acceptance,
marked #219 ready and submitted it to the protected queue without bypass or
branch deletion. Queue entry `MQE_lQDOTRPZ888AAAABG7MkdM4AA_LZzgMrAGo` is first.
Merge-group run 37822983601 is active at
`6d5dd32f6504c732b9431df8a11a8c8f84739e8e`; watcher 40642 observes it every
60 seconds. Submission is not a completed merge. Browser client 7643 is idle.

Next: collect 40642, verify remote MERGED state and integrated tree, then rebase
only #248's three owned commits from prepared base `8cef61a0` onto actual
master. Update its prepared receipt and publish with exact lease `a6d2e091`.
Preserve #212 and other prepared branches; no new PR or runtime activation.

### Permission domain merged; specification delivery published — 2026-10-08

#219 merged at 18:17:41 UTC as
`6d5dd32f6504c732b9431df8a11a8c8f84739e8e`. Protected merge-group 37822983601
passed; watcher 40642 is finished. Remote master tree exactly matches Nixbot
510's accepted `117adb9bb1583481a417d944265770217f90cab6`. Final receipt is
published in #219. Its branch and #212 remain intact; no runtime policy is active.

Fetched actual master and preserved local #248 `f8f029fd` at
`refs/backup/pr248-before-domain-merge-20261008`. Rebased only its three owned
commits from prepared #219 base `8cef61a0` onto `6d5dd32f`, with automatic ref
updates disabled. New head `76b767503994c8a0d8d68ba1aefcadd06b5eb0de` has tree
`e3a656f4b276149826c76ac160b0c68393d9c540`. No conflict; range-diff confirms
all three unchanged patches and whole-tree diff against `f8f029fd` is empty.
Existing exact-tree formatting remains valid; whitespace check passed.

Published only #248 with exact lease against `a6d2e091` and updated its body
to the real head/base. It remains draft pending GitHub 37823408244 and Nixbot
513, started at 18:19 UTC. No source/spec decision changes or dependent branch
rewrites. #223/#221/#222 remain locally prepared, not newly published.

Nixbot 513 stores exactly the expected tree `e3a656f4b276149826c76ac160b0c68393d9c540`;
its compilation/test attributes are building and browser/VM checks are pending.
GitHub watcher 83691 observes run 37823408244 every 60 seconds. These are
current checks, not assumed reuse merely because the owned diff is documentation.

Next: collect #248's current checks, then protected
queue and final-tree verification. After its merge, reconcile only #223's
owned commit onto actual master. Keep #212 unchanged and create no new PRs.

### Converter delivery receipt prepared while specifications validate — 2026-10-08

The preceding iteration made progress by verifying #219's merge and publishing
#248. Revalidated #248 head `76b76750`, draft/mergeable state, exact Nixbot 513
tree `e3a656f4` and live GitHub run 37823408244 (watcher 83691). GitHub formatting
passed in 58 seconds; full checks remain in progress. No new retry or queue entry.

Revalidated #223's clean local `8a9efae5` preparation and unchanged remote
`68661980`. Its two owned Rust files remain byte-identical; whitespace checks
passed. Prepared `.scratch/pr223-merge-delivery.md` with the reviewed lock-order
boundary, five retained races, unchanged query-cache accounting and historical
versus combined-tree acceptance clearly distinguished. This body remains local
until the one owned commit is reconciled onto actual master after #248 merges.
No new source edit, review claim, database operation, branch push or extra CI.

Nixbot watcher 26568 observes build 513 every 60 seconds and exits at a terminal
result; it has no restart logic. Its initial read confirms the expected tree and
active checks. GitHub watcher 83691 remains the corresponding Actions handle.

Next: collect #248's existing GitHub/Nixbot checks, then protected delivery and
tree verification. Afterwards publish the already prepared #223 using its known
remote lease; keep #212 and every prepared source branch intact.

### Remaining foundation descriptions prepared — 2026-10-08

Prepared local delivery descriptions for #221 and #222 in
`.scratch/pr221-merge-delivery.md` and `.scratch/pr222-merge-delivery.md`.
They distinguish the already recorded bounded reviews and local core checks
from still-required full current-tree acceptance. Both clearly state that the
application does not yet use or activate these extracted permissions. Rechecked
clean local heads/trees and whitespace; no new source review or test execution
is claimed by this documentation step.

Remote read-back shows both PRs already target master (their old body text still
describes the former stack). Their heads remain `37b124be` and `e8d89b46`;
prepared local heads remain `c05533c1` and `b0acd09f`. Do not retarget them
unnecessarily or publish either description before reconciling actual heads.

#248 remains draft/mergeable at `76b76750`, with no reviews or review threads.
Master is still `6d5dd32f`. Existing GitHub 37823408244 and Nixbot 513 watchers
remain live; formatting/evaluation passed, with no failure reported. No retry,
queue submission, branch publication, migration or additional PR occurred.

Next: collect those existing checks, queue #248 only after full acceptance,
verify the merged tree, then reconcile and publish #223. Preserve #212.

### Branding save review and local delivery preparation — 2026-10-08

The preceding turn made progress by preparing #221/#222 descriptions and was
also a verified wait on the same live #248 checks. Refreshed the open-PR
inventory without publishing branches or starting builds. Continued with the
independent #225 write boundary while the existing CI runs.

Using the repository Rust/async/minimal-change review guidance, traced the
authenticated wrapper, complete branding helper, current role/activity writer
lock order, post-commit event and all ten branding regressions. Compared the
bounded T071–T073 contract. Current tenant/activity/role checks apply to changed
and unchanged saves; organization-before-actor order and explicit READ COMMITTED
handle both lock waits. Authority remains locked through commit; failures roll
back and release locks. No critical/high source finding within this write
boundary. Other readers, CompanyWrite mapping and policy activation remain out
of scope. This was source review, not a new database-test execution.

Saved published `edd44094a7c83a9852b0b7fcf11a78affc67de9e` at
`refs/backup/pr225-before-delivery-chain-20261008`. Rebased its one owned commit
from `ed286e6b` onto prepared #222 `b0acd09f`, with updateRefs disabled.
Local head `3ae5c1f8eee5c2ad9ab84b5282eceb38e00ff319` has tree
`9268b6bf493417c9475b41bf011a8587364c3346`; no conflicts. Both Rust files and
all six original SQLx blobs still match the published head and original
`907bc88`. Range-diff only drops two descriptors already inherited unchanged:
`6ec0b36d` (REPEATABLE READ default) and `bcd26390` (READ COMMITTED transaction).
The owned diff is now six files, 402 additions/12 deletions. Nix-shell Cargo
format and whitespace checks passed; worktree clean. No database, migration,
cache regeneration, push or remote build was performed. A local delivery body
is prepared in `.scratch/pr225-merge-delivery.md`; update it after reconciliation
onto actual master. This preparation adds no functional dependency on #222.

Nixbot 513 now accepts all x86 attributes and the ARM package, core/server
tests, Clippy and SQLx checks. ARM browser/deployment/OIDC remain building;
GitHub 37823408244 Flake Check remains live. No failure or retry recorded.
Next: collect #248's existing watchers 26568/83691 and merge only after full
acceptance; continue #223, #221, #222, then prepared #225. Keep #212 unchanged.

### Specification GitHub checks accepted; ARM execution continues — 2026-10-08

The preceding turn made concrete progress reviewing/preparing #225. This
iteration verified the existing #248 processes repeatedly without restarting
them. GitHub run 37823408244 now succeeded: Flake Check 23m22s, Format 58s;
watcher 83691 exited successfully and must not be restarted.

Nixbot 513 remains live on tree `e3a656f4`. All x86 attributes and ARM
package/tests/Clippy/SQLx/formatting are accepted. ARM browser/deployment/OIDC
remain building, not failed. Two raw-log observations several minutes apart
showed new passing browser cases and guest boot progress; the deployment guest
now logs Horae listening. This is progress within the check, not terminal
acceptance. No retry, source change or merge-queue request occurred. The #248
description now links the accepted GitHub run and pending exact-tree Nixbot
build. Keep #248 draft until full acceptance; collect watcher 26568 next.

### Specifications merged; converter repair published — 2026-10-08

#248 completed all checks without retry: GitHub 37823408244 succeeded and
Nixbot 513 accepted 16 attributes plus four cached. ARM deployment completed
the full restart/repeated-import script in 1766.17 seconds. Both old watchers
83691/26568 are terminal. Verified current head/base, no review threads or
review blockers, then submitted to the protected queue without bypass.
Merge-group 37829252481 passed on the identical accepted tree.

#248 merged at 19:06:20 UTC as
`036aeebfb4877c572aa0f9d669f420a276511b61`; actual master tree is exactly
`e3a656f4b276149826c76ac160b0c68393d9c540`. Final receipt is published in its
description. This delivers documentation, not permission activation. Queue
watcher 35024 exited successfully; no branch was deleted.

Reconciled only #223's owned commit from prepared base `f8f029fd` onto the
actual master. Backup `refs/backup/pr223-before-specification-merge-20261008`
preserves `8a9efae5`. New head
`1a06d55f1fe2c28e1037d075fd713b08d5f8e6d1` has unchanged complete tree
`cc17441f8aea988af103e9568b4928d9045811c7`; range-diff identical, no conflicts,
whitespace clean. Published with exact lease against `68661980`, updated its
description to the actual integration, and left it draft pending acceptance.
No source/test/SQLx content changed and no dependent branch was rewritten.

Current #223 checks started: GitHub 37829568177 (watcher 76044) and Nixbot 514
(read-only watcher 61793, every 60 seconds, no restart logic).
The Nixbot stored tree matches `cc17441f`; builds are live, not stale acceptance.
Remote #212 remains open/draft at `db3935db364f2a8aa193f0e938ce40ecc01a2f92`.
Next: collect #223's current checks, deliver through the queue after acceptance,
then reconcile #221/#222/#225 in order. Ledger updates remain local on #218.

### User-write review and isolation regression in progress — 2026-10-08

While #223's existing CI runs, reviewed #227's three user-write paths, trusted
actor provenance, last-administrator checks, post-commit events and all seven
authority regressions plus existing concurrency cases. Found an unverified
isolation-default concern: the shared access transaction does not explicitly
choose READ COMMITTED, so the last-admin count may use a pre-wait snapshot
when the connection defaults to REPEATABLE READ. Do not certify or merge #227
until the actual regression result is known and any confirmed defect is fixed.

Preserved remote `fb63b766175359811d71d97be2134a2813e4c5fa` at
`refs/backup/pr227-before-delivery-chain-20261008`, then rebased its one commit
onto local #225 `3ae5c1f8` without automatic ref updates. Local committed head
`3eb8f9244beca4b1dc3e19d868253ac6791034db`, tree
`4ecfc6ceb8f0b22edf5ed397fe1ae2bb062ccd49`. No conflict: range-diff only reflects
the inherited backend-PID descriptor and #240's safe identity-list context.
The new regression is an uncommitted edit to `users/tests/concurrency.rs`;
no production fix or branch publication has occurred.

The regression reuses the real concurrent last-admin helper under a REPEATABLE
READ connection default, exercising all four demotion/deactivation pairings.
Diagnostic `.scratch/pr227-isolation-check.nix` runs the user tests in a Nix
sandbox with its own ephemeral PostgreSQL, not a real/development database.
Initial local session 62551 was deliberately stopped to correct a misplaced
Cargo argument before test execution; it is not a failing regression result.
Corrected session 73585 is live, compiling the actual application/test code.
Derivation: `hjprljs15lshqf922xj4wn7ragbin5kc-horae-user-authority-isolation-check-0.1.0`.

Latest #223 read-back: GitHub 37829568177/watch76044 and Nixbot514/watch61793
remain live. All x86 Nixbot attributes pass; ARM package/tests and dependent
browser/VM checks remain outstanding without failure. Next: collect 73585 and
the existing remote checks. Keep #212 and all published source heads untouched.

### Last-admin isolation repair verified locally — 2026-10-08

The #227 regression confirmed the concern: session 73585 exited with 14 tests
passing and one failing because concurrent self-demotions left zero active
administrators, not one. This was the actual application transaction against
disposable PostgreSQL, with REPEATABLE READ configured on test connections.

Added explicit READ COMMITTED before acquiring the organization lock in
`begin_user_access_change`. The six-line shared fix preserves predicates and
lock order and covers creation, role changes and activation. Its 29-line
regression reuses existing blocker-based synchronization for all four pairs
of demotion and deactivation, without timing sleeps or new dependencies.

The post-fix Nix run (session 12774) passed all 15 users tests, zero failed or
ignored, in 8.64 seconds after compilation. Derivation:
`5adxaycijg7022afsj66ys9lcsi5wgl1-horae-user-authority-isolation-check-0.1.0`.
Full SQLx preparation verification (session 98386) also passed with server and
all targets against its own migrated disposable database, with no cache delta:
`6c7ydxbb1lz642wca5by9w6dxylvsi7n-horae-sqlx-prepare-0.1.0`.
Pinned-Nix Cargo formatting and whitespace checks passed. No real data changed.

Saved the repair separately as unsigned local commit
`96fa8d1aa0a19342ba05cb8f56cd3d399e9d70bd`; tree
`bc7804d9cb5b874a59b399effa7349ef09bbc5d1`, on prepared #225 `3ae5c1f8`.
The worktree is clean. Remote #227 still has `fb63b766`; no publication or
current-head remote acceptance is claimed. Prepared delivery description is
`.scratch/pr227-merge-delivery.md`. Renewed bounded review found no remaining
critical/high issue in this user-write boundary after the demonstrated repair.

#223 GitHub run 37829568177 is now successful: Flake Check 25 minutes, Format
41 seconds. Its description records this evidence. Nixbot 514 remains live on
the exact expected `cc17441f` tree, with only ARM browser/deployment/OIDC checks
outstanding. Browser logs contain passing cases; no retry has been requested.
Next: collect watcher 61793, then use the protected queue only after acceptance.
Continue #221/#222/#225/#227 in order, reconciling onto actual master and
requiring fresh complete remote checks. Preserve #212 unchanged.

### Import delivery VM startup failure and isolated retry — 2026-10-08

Nixbot514/watch61793 completed with 18 attributes accepted and two failed:
ARM deployment and OIDC. Both logs fail at the initial driver-connection wait,
after 900.61 and 900.64 seconds respectively, before functional assertions.
The applications had started listening inside both guests. This establishes
the failed boundary, not the root cause of the driver delay or a source defect.
Derivations: `qlpdpbqlk5kq8wv9vafdj8dk4i7ya1p0-vm-test-run-horae-e2e`
and `ca3kfsvvgfsgx5ys7w27y4rahmqw719g-vm-test-run-horae-e2e-oidc`.

Reused the existing authenticated Nixbot Chrome connection. After checking
origin, exact `cc17441f8aea988af103e9568b4928d9045811c7` tree and the two failed
attributes with every other attribute accepted, sent one deployment-only
restart request. Public read-back confirms that attribute is now building;
OIDC remains failed and was not restarted. Do not duplicate the request while
the browser response is pending. Read-only watcher95726 polls the deployment
attribute every60 seconds and has no restart logic. #223 remains draft.

Next: collect that retry, then consider one isolated OIDC retry only after the
deployment result is known. Require complete acceptance and protected queue
verification before merge. No source modification or new PR was created.
The #227 description now explicitly distinguishes published `fb63b766` (known
isolation defect) from verified local `96fa8d1a`; it remains draft and unpushed.
Remote #221/#222/#225 heads remain unchanged with no new submitted reviews.

### Project-access delivery prepared on the repaired user boundary — 2026-10-08

Confirmed #223's isolated deployment restart returned HTTP200 in the existing
Chrome session. Watcher95726 remains live; OIDC is still failed and has not been
retried. The deployment guest is progressing through PostgreSQL initialization.
No second restart or extra remote build has been submitted.

While that check runs, renewed #228's bounded project-family review against
T024–T026/T068–T070. Traced all creation-actor and task-link production callers,
organization-before-actor/resource ordering, final gate modes, assignment
tenant discovery/recheck, revision triggers and real editor/revocation,
invoice-FK and time-entry-cascade regressions. No critical/high issue identified
within this bounded extraction; full T042 and other writers remain separate.

Preserved published `55382bd7c731f704b67dcf81a9f6d90a7a0e0d9b` at
`refs/backup/pr228-before-delivery-chain-20261008`, then rebased its single owned
commit onto corrected local #227 `96fa8d1a`, with automatic ref updates disabled.
Local head `34be6010cbe0e746f84a01497c13135cd9a4610b`, tree
`ced533319817a18e2d2edbcf4a5b0b0fddfb47d5`; worktree clean, no conflicts.

All15 Rust files remain byte-identical to the published extraction. Fourteen
match original `3ae8e08`; the remaining file retains already-integrated #216
client validation. Every added/replacement SQLx descriptor is preserved;
range-diff only removes five already-inherited descriptors from the owned patch
(`07580408`, `33678a5f`, `54b7cb41`, `7d778a4c`, `832f8ead`). Owned diff:
39 files,1700 additions,84 deletions. Pinned-Nix Cargo formatting and whitespace
checks passed. No migration, policy, UI, dependency or real-data change occurred.

Started the complete native Nix test check on this prepared tree with two cores
and one job. Session2783 is live, derivation
`h9y3nnaajh49vvpykmkglkshsx96505y-horae-tests-0.1.0`; its PostgreSQL is disposable
inside the Nix sandbox. The result is pending, not acceptance. Prepared local
description: `.scratch/pr228-merge-delivery.md`. Remote #228 remains unchanged
and draft, based on #227. Next: collect2783 and #223 watcher95726; reconcile
#228 onto actual master only after #227 integration and require fresh full CI.

### Deployment retry failed; OIDC retry and native suite active — 2026-10-08

#223 deployment watcher95726 exited failed: the one isolated retry again
timed out waiting for the test-driver readiness signal, after900.79 seconds.
The retry log shows Horae listening by guest time404 seconds, PostgreSQL still
checkpointing later, and virtual console setup completing at854 seconds.
It never reports the required controller connection before the900-second
deadline. No functional deployment assertion ran. Do not submit another
unchanged deployment retry or claim that serial execution solved this issue.

Read the pinned NixOS instrumentation and test driver at nixpkgs source
`ifpab9hxqmk2biwy594da8ipxzsp3y4s-source`: the controller service waits for
`dev-hvc0.device` and the architecture's serial device before its readiness
message. Logs show those device expectations and the built-in virtio-console
module, but do not establish why readiness is delayed. Searches of primary
upstream sources did not identify a matching issue; no upstream fix is claimed.
No timeout, required check, app code or deployment configuration was changed.

After the failed deployment terminated, requested the first OIDC-only retry
through the existing Chrome connection, guarded by exacttree and failed-state
checks. HTTP200 and public BUILDING state are confirmed. Read-only watcher60653
is live at60-second intervals; it never restarts work. The browser call finished
normally. #223 remains draft with its current failure and retry state published.
Deployment has now had two attempts total; OIDC's second attempt is active.

Evaluated `.scratch/pr223-arm-vm-local.nix` as a dry run only: existing ARM
guest test with native x86 controller. It lists251 build derivations; this host
advertises no aarch64 extra platform and has no ARM binfmt registration. Do not
claim an ARM reproduction or start that large build as a routine check. No host
configuration was modified and no VM/build was launched by the dry run.

#228 native session2783 passed all189 core tests and completed server compilation
in12m43s; the886 server tests are now running. Result remains pending. Next:
collect2783 and OIDC watcher60653, retain the deployment failure as a merge
blocker, and investigate with new evidence rather than automatic retries.

### Independent domain delivery published while import CI is blocked — 2026-10-08

#221 functionally depends on merged #219, not the import-report repair in #223.
Revalidated actual master `036aeebf`, the unchanged published #221 `37b124be`,
and its seven pure-domain files. Remove the delivery-order-only dependency so
the recurring #223 deployment-startup failure does not stop unrelated delivery.
#223 remains open/draft with its repair preserved and required checks unresolved.

Preserved the earlier local preparation `c05533c1` at
`refs/backup/pr221-before-independent-delivery-20261008`. Rebased only its owned
commit directly onto master, disabling automatic ref updates. New head:
`a549a89e19bcb1ee9e8504b870d7a68f0d5dd805`; tree
`f8c601ada1fbe112fdf188a69115d669f2ffe9ab`. Range-diff is identical; all seven
owned blobs are unchanged, with1179 additions and no extra source changes.
Fresh pinned-Nix validation passed:187 core tests, all-target core Clippy with
warnings denied, Cargo formatting and whitespace checks. Worktree is clean.

Published with an exact lease on remote `37b124be`; GitHub read-back confirms
the new head, master base and draft status. Updated #221's description with its
independent scope and current verification. GitHub run37837568734/watch66738
and Nixbot515/watch85217 are live. Nixbot's stored tree matches `f8c601ad`.
No protected merge has been requested and full native/ARM checks still apply.

No dependent branch was rewritten. #222/#225/#227/#228 still retain their prior
local prepared compositions through #223; reconcile only their owned commits
onto actual integrated master before their respective publication. Do not treat
old prepared-tree acceptance as current acceptance after changing that base.
Next: collect #221's new checks, #223 OIDC watcher60653 and #228 native2783.

### Complete prepared project-access native suite passed — 2026-10-08

#228 native session2783 completed with exit0 on unchanged prepared head
`34be6010cbe0e746f84a01497c13135cd9a4610b`, tree
`ced533319817a18e2d2edbcf4a5b0b0fddfb47d5`. Derivation
`h9y3nnaajh49vvpykmkglkshsx96505y-horae-tests-0.1.0` passed189 core tests,
875 server tests and183 tests in auxiliary/integration binaries, with zero
failures and11 pre-existing ignored manual server cases. Server tests took
345.91 seconds after12m43s compilation. The actual editor/revocation,
invoice/assignment-FK, time-entry cascade and #227 last-admin isolation
regressions all pass within this composition. Disposable PostgreSQL only.

Updated the local #228 delivery description with that evidence and its limit:
this prepared tree still includes pending #223, whereas #221's new independent
delivery does not. Reconcile before publication and require current-tree
acceptance; this is not full flake/browser/VM/ARM acceptance. No additional
source edits or remote build were needed. #221's GitHub66738/Nixbot85217 and
#223's OIDC60653 remain active; deployment514 remains failed after its one retry.

The isolated OIDC retry on #223/Nixbot514 also ended at the same driver
connection timeout (900.92 seconds). Neither unchanged ARM VM was retried
again. Published diagnostic-only commit ce371deb9e75fd02e9d8c84d512822d6c0e09245
on the existing #223 branch, using the exact previous-head lease 1a06d55f;
tree ca8017ec57930295a3843e0f76488cd849a55c2a. The PR remains draft.
Its 32 added lines in the two VM checks record pending jobs and console-device
state before the driver connects. Application code, deployment module,
assertions and timeouts are unchanged; this is not a claimed ARM fix.

Both complete native VM checks passed on that diagnostic source. Deployment
derivation q1lvib05yhn9vfpva79xr5n4m17wwdlj finished its script in129.69 seconds;
OIDC derivation k8ahz1x6fdxxxhk1ck29qjd4pcx8yh0j in73.43 seconds. Their logs
confirm the diagnostic service ran and emitted device state. Formatting and
whitespace checks passed. No real database was used. The published PR body
distinguishes the old failed build from the new diagnostic revision.

#221 remains the independent next delivery at a549a89e, with GitHub
run37837568734 live and Nixbot515 building its four remaining ARM attributes;
no failures have been observed in that build. Next: collect that acceptance
and submit to the protected queue if green, while collecting fresh #223 ARM
diagnostics separately. Preserve #212 and all downstream unpublished repairs.

Prepared #222 independently of #223 while #221 checks finish. Backup
refs/backup/pr222-before-independent-delivery-20261008 retains b0acd09f.
Rebased only the two storage/documentation commits from c05533c1 onto published
#221 a549a89e, with updateRefs and signing disabled. New local head is
82f3e7064a0046db79b0bba83f1eb32d0e1ca117, tree
9678d00463b7257da0014ccd2d9682ee4a890d6c. Both range-diff entries are identical
and every owned file preserves its previous blob. The complete-tree difference
is exactly the absent #223 repair. No downstream ref or remote #222 changed.

Fresh native tests and SQLx validation are running in disposable Nix sandboxes
on this independent composition (derivations drvzbzz0g8kkgrl9qvxi90vb761h3psh
and blyywdda1ilpr43fgr6yk99y2jrwwxjp). Do not claim acceptance before completion.
Reconcile onto actual integrated master before publishing #222. #223 now has
GitHub run37840158446 and Nixbot516 on the exact diagnostic tree ca8017ec;
#221/Nixbot515 has accepted everything except its two still-running ARM VMs.

#221 is accepted on its exact published tree: GitHub37837568734 passed Flake
Check in24m53s and Format in1m35s; Nixbot515 succeeded with16 built and four
already-built attributes, including both ARM VMs, without retries. Fresh
preflight confirms master036aeebf, unchanged a549a89e head, no merge conflict
and no outstanding reviews/threads. Published the acceptance body, marked the
PR ready and submitted it to the protected squash queue without bypass.
Entry MQE_lQDOTRPZ888AAAABG7ZUBM4AA_LZzgMrtG0 is first and QUEUED; merge-group
checks and actual integration are still pending. This is not a completed merge.

#223 GitHub37840158446 passed (Flake3m4s, Format49s). Nixbot516's ARM OIDC VM
passed its complete script in441.04 seconds. Diagnostics show real console
devices existed while systemd still classified them as tentative, before the
driver eventually connected. The root cause of intermittent earlier startup
timeouts remains unproven. Only ARM deployment is still building on516.

#222 native SQLx preparation passed and189 core tests passed on independent
82f3e706; the full server suite is still compiling in its existing build.
Next: follow #221's queue check and verify the actual merge tree, then reconcile
the next prepared delivery onto that master without changing owned source.

#221 merged through the protected queue at2026-10-08T20:46:32Z as
7212fc89299668f135fd51f212b67acdfdd420da. Merge-group37841801227 passed Flake
Check in55s and Format in40s. Read-back confirms MERGED and actual master tree
f8c601ada1fbe112fdf188a69115d669f2ffe9ab, exactly the accepted source tree.
No branch was deleted. #212 remains OPEN/draft at its original db3935db head.
Next: collect the running #222 native server suite before reconciling its
worktree onto actual7212fc89 master; #223/Nixbot516 still has its live ARM
deployment VM executing recovery assertions. Keep both existing checks running.

#223/Nixbot516 completed successfully on diagnostic tree ca8017ec: six built
and14 already-built attributes, including both ARM VM checks, without retries
of that revision. Only after collecting that terminal result, published the
prepared rebase onto actual master7212fc89. New head
6986dfcd6630f26d2b7450cf90b706a7b7fedeb0, tree
128e9df1022f53e587599463352795f2e7d60f96; exact push lease ce371deb preserved
the remote boundary. Backup refs/backup/pr223-before-domain-rules-20261008
retains ce371deb. Both range-diff entries and every owned file are unchanged;
the only whole-tree addition is the seven files already merged in #221.
Full formatting passed (499 files, zero changes) and whitespace checks passed.
No dependent refs, source assertions or timeouts changed. #223 remains draft
pending fresh combined-tree checks and protected-queue verification.

#222's existing native test process is still running; do not restart it or
change that worktree until completion. SQLx and189 core tests already passed.
Next: collect its server/integration result, reconcile its two unchanged owned
commits onto7212fc89, and publish the existing #222 branch with its exact lease.

#222's native build completed successfully:189 core tests,847 server tests and
183 auxiliary/integration tests passed (1,219 total, zero failed,11 existing
manual cases ignored). All11 permission-storage regressions passed, including
legacy preservation, tenant isolation, malformed grants, concurrent equivalent
names and Unicode migration rollback. SQLx preparation already passed on the
same independent tree. Both databases were disposable Nix-sandbox instances.

After the process exited, reconciled its two owned commits from a549a89e onto
actual merged master7212fc89. Backup
refs/backup/pr222-before-domain-merge-20261008 retains82f3e706. Range-diff is
identical and the complete tree remains9678d00463b7257da0014ccd2d9682ee4a890d6c.
New head26fe6075ca60b468bbb113925448be8578014225 passed full formatting
(502 files, zero changes) and whitespace validation. Published to the existing
#222 branch using exact remote lease e8d89b46 and updated its acceptance body.
It remains draft until fresh GitHub and both-Linux Nixbot checks pass.

#223's current head6986dfcd is independently running GitHub37842680338 and
Nixbot517 on tree128e9df1. Its earlier green516 result is not substituted for
that current composition. Next: collect the two published deliveries' checks,
then reconcile any changed base before protected-queue delivery. No new PR,
feature, real-data operation or downstream branch rewrite was performed.

While the two published builds run, completed a read-only bounded review of
#226 at769a0d8808dc88ca880556ba4d38374167d0412f, relative to its actual owned
base e8d89b46 (not a moving branch name). Read its321-line command module,
transaction configuration/loaders, DTOs, receipt migration, all19 command tests,
the real PostgreSQL blocker helper and template-commands contract. Verified
the command/test/migration blobs match the preserved #212 reference exactly.

No high/critical finding within this internal create/delete boundary: explicit
READ COMMITTED/READ WRITE precedes the organization gate; current active actor
and canonical administrator identity are checked before receipt lookup; tenant
scoping, canonical idempotent replay, count50, strict grants, checked increments,
detachment preservation and receipt/audit rollback are retained. Row order is
organization, actor, template, sorted person states. Transaction-local limits
preserve stricter pool timeouts. The19 tests include real blocked revocations,
concurrent retries/count admission and injected audit failure. No mutation
endpoint or policy activation is introduced, and no production caller exists
in this extraction. This is source review, not fresh runtime/ARM acceptance or
closure of the cross-feature lock audit. No #226 file or branch changed.

Current published checks remain live without observed failures:
#222 GitHub37843072424/Nixbot518 on9678d004; #223
GitHub37842680338/Nixbot517 on128e9df1. Next: finish those acceptance gates and
queue the next ready delivery; retain #226's review for its later reconciliation
onto integrated #222. No extra remote build was started for #226.

Prepared #226 locally on the currently published #222 while existing remote
checks continue. Backup refs/backup/pr226-before-storage-delivery-20261008
retains769a0d88. Rebased only its one owned commit from e8d89b46 onto26fe6075;
new head b64ae15c1e4884089162afde2cf3e6196a54e0c0, tree
4e1ba3e060cc9fa1faa96d3bbd2372691c826e31. Range-diff is identical and every
owned file retains its blob, including41 SQLx descriptors and receipt migration.
The scoped diff remains47 files,2,147 additions and one deletion. Full formatting
passed (505 files, zero changes), as did whitespace checks. No dependent ref or
remote #226 changed, and no new source code or test assertion was introduced.

Native full tests and SQLx validation are running sequentially in existing Nix
checks, one build job/two cores and disposable databases. Derivations are
j5ycwj3k0mg6l74v2v0pickimbjfzrcr-horae-tests-0.1.0 and
yc2zffpi1w2n5l7zvk5wdcayp9p2rci5-horae-sqlx-prepare-0.1.0. Sixteen GiB were
available at start; no cleanup or real-database change was performed. Do not
modify this worktree while its native validation runs. Next: collect the live
#222/#223 remote checks and queue a ready delivery; retain #226 locally until
its predecessor is integrated and its own acceptance is complete.

Verified the repository's actual queue contract before further delivery:
master requires Flake Check and Format, allows squash only, and uses an
ALLGREEN merge queue (up to five entries). The checked-in merge_group workflow
runs full native `nix flake check -L` and `nix fmt -- --ci` on the temporary
combined commit. Branch protection does not require an artificial source-branch
rebase after every independent merge. Both #222 and #223 were already rebased
onto master7212fc89 containing the repaired CI baseline.

A non-checkout `git merge-tree --write-tree` of published #22226fe6075 and
#2236986dfcd succeeds without conflicts; expected combined tree is
373150c401ec7bbe32db3bf3ce466cfcc7701369. No branch or working file changed.
Once each head passes its own GitHub and both-Linux Nixbot checks, the queue
can validate this independent combination without discarding completed CI just
because the other delivery merged first. Verify the actual queue tree and
results; rebase if a conflict, dependency change or new source difference
requires it. Do not describe per-head ARM checks as a new combined-tree ARM
execution, and do not bypass or weaken any required gate.

#226's native validation completed successfully on b64ae15c/tree4e1ba3e0:
189 core,866 server and183 auxiliary/integration tests passed (1,238 total,
zero failed,11 existing manual cases ignored). The server suite took161.08
seconds after4m04s compilation. SQLx preparation also passed, with no cache
delta. Updated the local delivery body with this exact-tree evidence; #226
remains unpublished and requires integration of #222 plus fresh remote gates.

Both current remote builds have now accepted everything except their live
ARM deployment VM: #223/Nixbot517 and #222/Nixbot518. Neither has reported a
failure or been retried. GitHub checks remain separately monitored at their
existing run IDs. Next: collect the final VM/GitHub results, submit ready heads
to the protected queue, and verify the actual merged tree against the accepted
individual tree or the precomputed373150c4 combination, as applicable.

#223 passed its current-head gates at 6986dfcd: GitHub 37842680338 completed
Flake Check in 20m55s and Format in 45s; Nixbot 517 accepted all 20 attributes
(16 built, four already built) on exact tree 128e9df1, including both ARM VM
checks, without retries of this revision. Fresh preflight found no conflict or
outstanding review, with master still at 7212fc89. Published the acceptance
body, marked #223 ready and submitted it to the protected squash queue.

Queue entry MQE_lQDOTRPZ888AAAABG7tw-M4AA_LZzgMr25c is first and awaiting
checks. Merge-group run 37845978321 is live on temporary commit d79a7d4e.
This is not yet a completed merge. #222 remains at its unchanged published
head with GitHub and its final ARM deployment VM still running. Next: collect
the queue result and verify actual #223 integration, then submit #222 only
after its own current-head gates pass; verify its combined tree against the
already checked 373150c4 combination instead of forcing an unnecessary rebase.

#223 is now confirmed MERGED at 2026-10-08T21:20:59Z as
d79a7d4e815422dbdf517f7613fcf1939a3d3f15. Merge-group 37845978321 passed
Flake Check in 43s and Format in 42s. The actual master tree is
128e9df1022f53e587599463352795f2e7d60f96, exactly the accepted #223 tree.
No branch was deleted or protection bypassed.

#222 also completed its current-head gates at 26fe6075: GitHub 37843072424
passed Flake Check in 24m44s and Format in 47s; Nixbot 518 accepted all 20
attributes (16 built, four already built) on exact tree 9678d004 without
retries. Published its acceptance body and requested protected-queue delivery
with the unchanged head. Its combined integration with #223 must pass the
queue checks and match expected tree 373150c4 before claiming a merge.

Stopped only the owned read-only monitor for completed builds 517/518; no build
was cancelled. #226 retains its complete passing local suite and unpublished
b64ae15c head. Next: follow #222's actual queue entry/run, verify integration,
then reconcile #226 and the remaining prepared deliveries onto the merged base.

#222 is first in the protected queue, entry
MQE_lQDOTRPZ888AAAABG7l00M4AA_LZzgMr38I, awaiting checks. Run 37846421902
is live on temporary commit c4aa27c1; its actual tree is exactly the expected
373150c401ec7bbe32db3bf3ce466cfcc7701369 combination with merged #223.
Format passed; Flake Check remains in progress. No source rebase or duplicate
remote matrix was requested for this independent combination.

Prepared and published the next independent repair, #225, on actual master
d79a7d4e. Backup refs/backup/pr225-before-independent-delivery-20261008 retains
3ae5c1f8. Rebased only its owned branding-authority commit from b0acd09f, with
updateRefs/signing disabled. New head 1bce0030f1b738c1dd8127f4f9d488e69058437c,
tree f352a52924eb0b3368f6a967f5baed1675b0d616. The patch is identical in
range-diff and all six owned files preserve their blobs: 402 additions and 12
deletions. Full formatting passed (499 files, zero changes) and whitespace
checks passed. No fresh local database acceptance is claimed on this tree.

Published with exact remote lease edd44094 and updated the existing PR body;
no new PR or dependent ref change. #225 remains draft until current-head
GitHub/Nixbot checks pass. #222 was only an earlier delivery-order predecessor,
not a functional prerequisite for branding authority. Next: collect #222's
queue result and verify integration; follow #225's fresh checks and retain the
passing local #226 preparation for reconciliation after its storage prerequisite.

### Combined queue validation verified — 2026-10-08

The preceding status turn was a verified wait: #222 remained open while its
specific merge-group job and Nixbot build were confirmed running. No duplicate
run or source change was requested.

Nixbot 519 automatically builds queue commit
`c4aa27c1e0aaf9d12ac9f7c89050d8a76993a2cd`, not merely #222's source branch.
Its stored tree is `373150c401ec7bbe32db3bf3ce466cfcc7701369`, matching the
previously verified combination with integrated #223. ARM browser, native
tests and both deployment VMs are running; x86 browser is also running. No
failure has been observed. GitHub merge-group 37846421902 remains live, with
Format passed. Integration and complete combined acceptance remain pending.

#225's GitHub run is 37846756514; Format passed, Flake Check is running.
Nixbot 520 is building the exact published tree
`f352a52924eb0b3368f6a967f5baed1675b0d616`, with no failed attribute observed.
These are independent deliveries with fresh checks, not unchanged retries.

Next: collect these existing jobs. Verify #222's actual merge before rebasing
and publishing #226. Preserve #212 and all source branches. The ledger remains
locally committed until its own existing documentation delivery is reconciled.

### User authority repair prepared independently — 2026-10-08

Rechecked #227's three production wrappers, shared transaction initializer,
last-admin guard, all authority/concurrency tests and the real database-blocker
helper. Its dependencies are already in master after #223; neither inactive
storage #222 nor branding #225 is required. Existing findings and their repair
remain unchanged. No new high/critical issue was found in this bounded review.

Saved `96fa8d1aa0a19342ba05cb8f56cd3d399e9d70bd` at
`refs/backup/pr227-before-independent-delivery-20261008`. Rebased only the two
owned commits from prepared #225 onto master
`d79a7d4e815422dbdf517f7613fcf1939a3d3f15`, with automatic reference updates
disabled. Both range-diff entries are identical; all six owned file blobs are
unchanged. Local head is `5b96629521ca31b3232740ac020db007bdbcf858`, tree
`de9c2f3dfd0db2d78785d266d2e41f7b5f022fe6`; 494 additions, 26 deletions.
Published #227 remains at `fb63b766`, and #228's prepared branch is untouched.

Started formatting, full native tests and SQLx preparation against disposable
Nix databases (session 79286), with one local build job and four cores. These
are fresh checks of the independent tree, not a rerun of a stalled process.
Wait for acceptance before publication; retain only the existing two active
remote deliveries until one finishes. #222 queue build 519 still has browser
and ARM VM checks running without a recorded failure; its ARM tests passed.

### Independent user-authority native acceptance — 2026-10-08

Session 79286 completed successfully on unchanged #227 head `5b966295`, tree
`de9c2f3dfd0db2d78785d266d2e41f7b5f022fe6`. Full native checks passed:
187 core, 849 server and 183 auxiliary/integration tests, totaling 1,219 passed,
zero failed and 11 pre-existing manual cases ignored. All 15 user tests passed,
including the four-case REPEATABLE READ last-administrator regression. The
server suite took 74.60 seconds. Full SQLx preparation passed without a cache
delta; formatting processed 500 files without changes, and whitespace passed.

Derivations: `kwgrlgif5gbpv3iy9fa0dll9hq0h7695-horae-tests-0.1.0` and
`95ziyjxxv7xj4s4b37vfdmbsl5yvqr33-horae-sqlx-prepare-0.1.0`. Only disposable
Nix databases were used. The worktree is clean, no local build remains active,
and the filesystem has 16 GiB free; no cleanup was needed.

#227 is still not pushed. Current remote read-back confirms its old draft head
`fb63b766`, #226's old draft head `769a0d88`, and unchanged open/draft #212
at `db3935db`. #222's queue and #225's own-head jobs remain live. Next: collect
those jobs, reconcile #226 after actual #222 integration, and publish the next
bounded delivery without exceeding the current two-build remote concurrency.

### Storage integrated and profile commands reconciled — 2026-10-08

The preceding iteration made progress by preparing independent #227 and
completing its full native/SQLx checks. Continued observation of the existing
jobs required no rerun. #222 merged through the protected queue at 21:48:10 UTC
as `c4aa27c1e0aaf9d12ac9f7c89050d8a76993a2cd`. GitHub merge-group 37846421902
passed Flake Check in 24m12s and Format in 45s. Its actual master tree exactly
matches the planned combination `373150c401ec7bbe32db3bf3ce466cfcc7701369`.

Nixbot 519 then completed successfully on that same combined tree, including
both ARM VMs and browser. The final deployment result arrived after the merge;
it has now been collected and is green. No retry or weaker assertion was used.
Watcher 79166 completed successfully. Fetched master without changing the root
checkout, and retained #222's source branch.

Rebased only #226's owned commit from published #222 onto the actual merge,
with automatic reference updates disabled. Backup
`refs/backup/pr226-before-storage-merge-20261008` preserves `b64ae15c`.
New local head is `1cbde180781c4bf11c429ee560bedccc897b0e97`, tree
`e910a2cbb31027e5f7ec7c2899112bd3f91c36f7`. Range-diff is identical and every
owned file retains its prior blob: 47 files, 2,147 additions and one deletion.
No source edit or dependent-branch rewrite occurred. The new base additionally
includes #223; the earlier 1,238-test native result remains previous-tree
evidence, not full acceptance of this final combination.

Next: publish and retarget existing #226 to master after formatting, then
collect its fresh GitHub/Nixbot checks. #225 remains the other active delivery;
#227's independently verified head stays local until a slot is available.
Keep #212 open and unchanged. No new PR was created.

### Profile commands published; branding ready for queue — 2026-10-08

#226 formatting passed on the final tree (505 files, zero changes), as did
whitespace validation. Retargeted the existing draft PR to master and published
`1cbde180781c4bf11c429ee560bedccc897b0e97` with an exact old-head lease against
`769a0d8808dc88ca880556ba4d38374167d0412f`. Remote read-back confirms master,
47 files and unchanged owned scope. GitHub 37849653227 is running on this head.
Fresh both-Linux Nixbot acceptance and protected-queue verification are pending.

#225's current-head GitHub 37846756514 passed Flake Check in 23m53s and Format
in 41s. Nixbot 520 passed the exact `f352a529` tree on both Linux platforms,
including ARM browser and both VMs, without retries. Watcher 45586 exited
successfully. No review blocker is recorded and the branch is unchanged.

After #222 integrated, merge-tree calculates the clean combination
`87ee2dd2aada51e08ea13ec26d1a7a9a062c9e2e`. Each side preserves its stable patch
ID: branding `5d8b3bcfe99db100e521c8edd7a22fd96447f2e0`, storage
`a3c6211ba1528d2bbb1608dca13b69a812c3a9e7`. No unnecessary source rebase or
duplicate individual-head build is needed. Submit #225 to the protected queue,
then verify its actual combined commit and checks; submission is not a merge.

### Branding queued; exact profile-command CI confirmed — 2026-10-08

#225 is ready and first in the protected queue, entry
`MQE_lQDOTRPZ888AAAABG8AcHM4AA_LZzgMr__4`, AWAITING_CHECKS. Its source head
remains `1bce0030`; no source rewrite or bypass was used. Merge-group run
37849797492 is live on `eb8eae55bf2f1f2defa5867447f950ea2404da6f`. The actual
tree was verified equal to `87ee2dd2aada51e08ea13ec26d1a7a9a062c9e2e`.
Format passed in 44 seconds; watcher 20198 follows Flake Check. Nixbot 523
is pending on this same combined tree and queue branch. Collect both queue
and Nixbot results before claiming completed acceptance.

#226 GitHub 37849653227 is live; Format passed in 43 seconds. Its read-only
watcher is session 77574. Nixbot 522 is building exactly
`e910a2cbb31027e5f7ec7c2899112bd3f91c36f7` on both Linux platforms. No failed
attribute was observed. The PR remains draft until acceptance completes.
Next: collect these existing jobs, confirm actual integration, and then publish
the independently verified #227 rather than start another feature or PR.

### Project lock-order delivery reconciled locally — 2026-10-08

The preceding turn made progress: #222 merged with complete native/ARM
acceptance, #226 was published, and #225 entered the protected queue. Current
checks remain live: #225 merge-group 37849797492 and Nixbot 523 (pending on
the verified combined tree), plus #226 GitHub 37849653227 and Nixbot 522.
No new remote build or retry was requested in this iteration.

Saved #228's prior prepared head `34be6010` at
`refs/backup/pr228-before-independent-delivery-20261008`, then rebased only
its one owned commit onto independently verified #227 `5b966295`. Automatic
reference updates were disabled. The rebase was conflict-free; range-diff is
identical and all owned file blobs are unchanged. Local head is
`38ac02c9e20e5b9df75ace22877f84205eccc35a`, tree
`a2371111c9877b20b99040e507b6138b814efd25`; 39 files, 1,700 additions and 84
deletions. No code edit or dependent-branch rewrite occurred.

Session 58298 is running fresh formatting, full native tests and SQLx
preparation with one local build job and four cores, using disposable Nix
databases. The former 1,247-test result belongs to the prior combined tree,
not this composition. #228 remains unpublished, as does #227. Next: collect
the existing checks, deliver #225/#226, then publish the prepared next repair.

## Objective and limits

Split the existing work in #212 and #217 into reviewable deliveries, preserving
all original changes. This is not implementation of the remaining permission
requirements or completion of Harvest parity. Do not merge or close #212/#217,
activate canonical policy, alter real data, or modify #208.

This is the single separation ledger. Existing feature contracts remain the
source of product requirements; do not restart or duplicate them.

## Pre-merge handoff — 2026-10-08 (historical)

This checkpoint predates the authorized delivery phase above. Its blocker,
draft-state and pending-authorization statements are historical, not current
instructions or current CI acceptance.

Completion is blocked on external ARM builder diagnostics. The failed #228 and
#270 checks were revalidated at their unchanged published heads; they are
terminal, not merely slow jobs. Other builds, including documentary build486,
remain live and have not been cancelled. Live work may produce additional
evidence, but does not repair these terminal failures or identify their cause.
The requested operator evidence and #283/#284 consolidation decision have not
arrived. Do not treat this handoff as completion or permission to merge.

The separation is published, but verification is not complete. All 57 code
extractions and the two documentation PRs (#248 and this ledger) now have
descriptions naming their published head, review base, preserved scope and
historical versus current checks. A final read-back verified all 59 descriptions
and draft states. The five separate shared-CI PRs bring the delivery inventory
to 64; the 18 integration-base branches are not additional delivery PRs.

The latest commit-specific CI audit spans 01:50–01:58 UTC. It retained 50 responses
after a connection reset and queried only the remaining 14 on continuation:

- Passed checks: #282, #284, #285 and #286.
- Failed: #228 and #270 have new ARM compilation failures. Both test and Clippy
  compilers received SIGKILL; package compilation exited 1. Logs do not establish
  OOM or who sent the kill. No blind build retry or source workaround was made.
- #283 still has its historical ARM VM failure on its own head; downstream
  #284–#286 pass, but their results do not turn #283's checks green.
- The other 57 PRs still await Nixbot build acceptance at their observed heads.
  #218 and #248 have passed GitHub Flake Check and Format on their published
  documentation heads; Nixbot is a separate gate.

Later attribute-level evidence at 02:15 UTC finds an ARM test-compilation failure
inside #220/Nixbot431 while its overall build is still running. Its rustc process
also received SIGKILL, on elastic-arm-234f8218. The aggregate pending count above
must not be read as absence of intermediate failures. The raw log does not
establish the cause of the kill; no retry or source workaround was introduced.

The broader attribute audit at 02:22 UTC supersedes that partial failure inventory:
15 of the 57 live builds already have failed ARM attributes (#220, #225, #227,
#238, #240, #245, #256, #268, #269, #272, #275, #276, #277, #278 and #280).
Across those builds, 20 retrieved logs show 15 compiler SIGKILLs, three explicit
1,200-second farm timeouts, one browser timeout, and one package compile failure
without an exposed signal. Nine other failed attributes never ran because their
package dependency failed. The failure categories must not be collapsed into
an unproven OOM diagnosis or reported as 15 independent product defects.

Full native checks already passed on the published #219/#220/#223/#224 heads
and on the complete cross-extraction composition `8fc3a44e`. That composition
passed 1,478 server tests (11 existing ignored cases), 15 auxiliary test binaries,
browser, Clippy, SQLx, formatting, package and deployment/OIDC checks. This is
not ARM acceptance or a substitute for individual current-head gates.

The shared prerequisite order is #282 → #283 → #284 → #285 → #286. The eleven
direct extraction roots and both documentation PRs are based on #286. Each
dependent PR describes its functional prerequisite order; integration branches
are review/test compositions, not merge targets. #220 carries the independent
legacy-reader repair from #217; do not integrate both overlapping deliveries.

Remaining work: resolve the failed/pending exact-head gates, finish the final
ownership and readiness audit, and publish this ledger checkpoint with the
delivery order and explicit incomplete work. No feature completion or policy
activation is claimed. #208/#212/#217 and original unpublished work remain
preserved; no merges or closures have been performed. Dated checkpoints below
retain historical hashes/results and must not override this handoff.

The final preservation bridge also passed: all 57 code review patches and all
56 changed documentation blobs remain identical across the CI rebases. The
original dirty worktree still matches its tracked snapshot and six-file backup
archive. All 30 unchecked source tasks have exactly one retained-work
classification; no checkbox was changed to claim feature completion.

## Current delivery order

This index names the actual delivery PRs rather than their temporary integration
branches. Each row requires its listed predecessors and their dependencies;
independent rows need not wait for unrelated features. Retain completed rows for
traceability; this is a dependency index, not a list of exclusively open PRs.
The active delivery phase above records current heads, gates and merge receipts.
Historical tables below do not certify a newer published head.

The shared-CI prerequisites are integrated: #282 and the consolidated #286
merged; #283–#285 were closed with their content preserved. Functional
deliveries #239, #242, #240, #220, #224 and #219 have also merged, as has
documentation #248. #217 was closed only
after #220's final tree was verified. Preserve #212 open and unchanged.

The next independent delivery is #221 (published directly on actual master,
fresh CI running). #223 remains separately blocked by recurring ARM deployment
startup failure; its isolated OIDC retry is active. After #221, reconcile #222
(local preparation, core tests/Clippy/format passed and bounded review complete),
followed by #225 (bounded review and local
rebase/format complete) and #227 (last-admin isolation repair, users tests,
SQLx and formatting verified locally), then #228 (source-preserving local rebase
on corrected #227; native tests running). Full current CI remains required. These preparation
bases are an integration sequence, not new functional dependencies. Reconcile
each onto actual master and verify its complete integration gates before merge.
The documentation deliveries #248 and #218 do not activate features.

| PR | Scope | Delivery prerequisites |
| --- | --- | --- |
| #219 | Pure permission scopes and catalog | #286 |
| #220 | Legacy report and invoice snapshot authority | #286 |
| #221 | Pure rate, management and approval rules | #219 |
| #222 | Inactive permission-profile storage | #219 |
| #223 | Import report conversion lock order | #286 |
| #224 | Interrupted import session cleanup | #286 |
| #225 | Current authority for organization branding | #286 |
| #226 | Internal reusable-profile commands | #222 |
| #227 | Current administrator authority for user changes | #286 |
| #228 | Project-access writer coordination | #227 |
| #231 | CSV preparation outside transactions | #286 |
| #232 | Financial snapshot reader authority | #220 |
| #233 | Invoice writer and revocation ordering | #228, #232 |
| #234 | Atomic person-profile commands | #226, #221 |
| #235 | Harvest connection change authority | #228 |
| #236 | Import job commands and error downloads | #235, #231 |
| #237 | Original import requester provenance | #236, #222 |
| #238 | Budget email recipient authority | #286 |
| #239 | Approval tenant isolation | #286 |
| #240 | Restricted identity response projections | #286 |
| #241 | Time-entry writer account activity | #228 |
| #242 | Time-entry invoice identity boundary | #286 |
| #243 | Project-manager delegation | #234, #228 |
| #244 | Own permissions in Settings | #234 |
| #245 | Permission change history | #243, #244 |
| #246 | Read-only legacy permission diagnostics | #237, #226 |
| #247 | Materialized spreadsheet and invoice PDF authority | #228, #232, #222 |
| #249 | CSV delivery authority | #247 |
| #250 | Requester-bound permission editor API | #245 |
| #253 | Scoped people directory reads | #245 |
| #254 | Identity-only project team choices | #253 |
| #255 | Scoped time-entry reads | #245 |
| #256 | Timesheet person discovery | #255, #253 |
| #257 | Requester/subject Timesheet context | #256 |
| #258 | Person-bound Timesheet commands | #257 |
| #259 | Selected-person Timesheet screen | #258 |
| #260 | People directory and permission editor UI | #240, #250, #254 |
| #261 | Detailed time reports and period totals | #257 |
| #262 | Grouped time report readers | #261 |
| #263 | Scoped time spreadsheets | #261, #247 |
| #264 | Scoped time CSV and shared download filters | #263, #249 |
| #265 | Permission-mode-bound time downloads | #264 |
| #266 | Grouped time CSV and spreadsheets | #265, #262 |
| #267 | Active-project and billability download filters | #266 |
| #268 | Scoped Reports screen | #267 |
| #269 | Protected project editing | #240, #250, #254 |
| #270 | Scoped project overview and detail | #240, #250, #254, #232 |
| #271 | Project export and monetary delivery authority | #267, #270 |
| #272 | Harvest-compatible project reads | #270 |
| #273 | Task catalog and tracking reads | #272 |
| #275 | Task creation and explicit rate edits | #273 |
| #276 | Task archive/restore and import preservation | #275, #269, #258, #223, #224, #231 |
| #277 | Project-task links and rate currency | #276 |
| #278 | Atomic task and initial-rate creation | #277 |
| #279 | Task catalog and editor UI | #278, #260 |
| #280 | Project task archive/restore controls | #276 |
| #281 | Harvest-compatible client reads | #272 |

The index contains all 57 code extractions exactly once. Its dependency graph
is acyclic and references no omitted delivery. Multi-parent entries name the
component deliveries documented by their review bases, not a claim that each
rewritten head is an ancestor. Integration branches are never merge targets.
After prerequisites land, retarget/rebase only the owned change and require
fresh checks for any new head. The combined-tree acceptance does not waive that
gate. The retained-work classification below separately covers unfinished
features, policy cutover and broad acceptance tasks.

## Verified starting state — 2026-10-06

- #216 merged at 13:16:59 UTC as `02f7b58acdcf126415f9ec89215da8cdada7d03f`.
  Queue CI run [37464771137](https://github.com/numtide/horae/actions/runs/37464771137)
  passed Flake Check and Format. This proves the base, not any extraction.
- #212 remains open/draft at `db3935db364f2a8aa193f0e938ce40ecc01a2f92`.
  Its original change set from `9301112c6a02ae3c92716273534f38889db241d1`
  contains 145 commits and 1,214 file changes, including 841 SQLx descriptors.
  Fifty commits change only specification/governance documents; 95 also change
  code, tests, or tooling. File counts are not delivery boundaries.
- #217 remains open/draft at `dd141c5cc4f8dea4f41a4c7cfbb724323ff6d198`,
  based on `feat/scoped-permissions` at `5faed76`. Its sole additional commit
  is not present in #212. It has no current-head CI.
- The worktree named `.worktrees/report-reader-authority` actually holds
  `feat/delegated-timesheet-commands` at `a0ea691`, not #217. Its commits
  `8360b20` and `a0ea691` are patch-equivalent to #212's `48a6533` and
  `02c4245`; preserve the original ref without extracting duplicates.
- Root `master` and the original worktrees were not reset or switched.
  The root's untracked `.playwright-mcp/` and ignored runtime/evidence files
  remain untouched in place.

## Preservation and recovery

Private local backup directory:
`.scratch/pr212-split-backup-20261006.BwoORH/`.

| Reference | Preserved object |
| --- | --- |
| `backup/pr212-split-20261006-original` | `db3935db364f2a8aa193f0e938ce40ecc01a2f92` |
| `backup/pr217-split-20261006-original` | `dd141c5cc4f8dea4f41a4c7cfbb724323ff6d198` |
| `backup/timesheet-split-20261006-original` | `a0ea69108416f2cc380d2145d9b21b44d4c0338f` |
| `backup/pr212-split-20261006-uncommitted` | `d364270a6c7684457734dff66ff54f54e1066f83` |

The uncommitted snapshot was created with `git stash create`, not a stash
operation that clears the index/worktree. No original edits were removed.

| Backup file | SHA-256 |
| --- | --- |
| `originals.bundle` | `b66db9b106948ca2d1bc8c28040fd945a76691d350f4b524b535ac3ee64aa305` |
| `unstaged.patch` | `d90b1797021e6d9d2fcb383d98d8db8993516675c887588a93203c6606fddc78` |
| `staged.patch` (empty) | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `untracked.tar` | `3b76bc925c05943a3732c4c2c5bdbb8ad58e1e90680390b3400c03b5b244aabc` |

Recovery verification passed: `git bundle verify`; independent no-checkout
clone of the bundle into `restore-check/`; checkout of the uncommitted snapshot;
extraction of all six untracked files; archive comparison against the source;
zero tracked diff between the source worktree and saved snapshot. Regenerating
the original-to-snapshot binary diff produced the exact saved patch hash.
The original worktree's status remains unchanged.

To inspect recovery, use the existing private `restore-check/` copy. For a new
recovery, clone the bundle into a new empty directory, check out the saved
snapshot and extract the untracked archive there. Never restore over an
original worktree.

### Original unpublished work: extraction and retained follow-up

All 18 original paths below remain preserved and unchanged. Eleven are now
extracted in draft [#281](https://github.com/numtide/horae/pull/281),
`feat/scoped-harvest-clients` at `cd8d1db7`: the six cache paths, four Rust paths
and `contracts/client-permissions.md`, plus the original shared-test visibility
hunk. Publication91690 succeeded after the transient GitHub errors. The other seven
specification paths remain retained with unfinished client-workflow follow-up:
dependent reconciliation, operation matrix, plan, spec, tasks, progress and
quickstart. Their approved FR-035/036 decisions are also preserved in the
extracted contract; their historical checks are not fresh extraction evidence.
T237 has current-head native extraction verification recorded below; T238
implementation remains incomplete. Do not treat
#216's legacy Clients MVP as canonical-permission acceptance.

| Original status | Path |
| --- | --- |
| ` D` | `.sqlx/query-63f2ec39d8821f1ba1534ddfb43cae8142e9b137e4d1c17d2e262e4ff66a272e.json` |
| ` D` | `.sqlx/query-cfb2a46e0259c64f406084030f1e3bb6e4ac424bc615880032a02ea0b7c07ad2.json` |
| ` D` | `.sqlx/query-f64bd1dac9e7dfaba8f8df5e299dc3370b32773b909a0ead9245492364e08aad.json` |
| ` M` | `crates/horae/src/harvest/mod.rs` |
| ` M` | `crates/horae/src/harvest/pagination_tests.rs` |
| ` M` | `specs/015-scoped-permissions/contracts/dependent-spec-reconciliation.md` |
| ` M` | `specs/015-scoped-permissions/contracts/operation-matrix.md` |
| ` M` | `specs/015-scoped-permissions/plan.md` |
| ` M` | `specs/015-scoped-permissions/progress.md` |
| ` M` | `specs/015-scoped-permissions/quickstart.md` |
| ` M` | `specs/015-scoped-permissions/spec.md` |
| ` M` | `specs/015-scoped-permissions/tasks.md` |
| `??` | `.sqlx/query-0565720011fa00b20930e95efb31d75581de152f75a5aad3a13ebdaa04b898e2.json` |
| `??` | `.sqlx/query-0574b2ae32e7f4df176d2bc39f263c8444e4cb526a8bd4f93199ebf633045590.json` |
| `??` | `.sqlx/query-f88711969f759f5466db07ed2dff6b46f0e353cd7015f004ace804a7516cec4d.json` |
| `??` | `crates/horae/src/harvest/client_reads.rs` |
| `??` | `crates/horae/src/harvest/pagination_tests/client_permissions.rs` |
| `??` | `specs/015-scoped-permissions/contracts/client-permissions.md` |

## Verification checkpoint before publication — 2026-10-08 (historical)

- #248's prior documentation headb6e13979 has green GitHub checks but failed
  Nixbot216 ARM VMs: OIDC shell readiness exceeded900s; deployment reached the
  application but its repeated-import wait exceeded90s. Its separate CI-base
  refresh is now staged at a6d2e091 over #286, preserving the exact56-file
  documentation patch; formatting passed and fresh CI is required. #218's
  ledger was also rebased, preserving every unpublished progress commit.
- Locally staged #219690cce20 and #220ea78c27c now have complete native
  exact-head acceptance, sessions16392/67402 respectively. #219 passed158 core
  and821 server tests; #220 passed121 core and828 server tests. Each passed nine
  auxiliary binaries, browser, Clippy, SQLx, formatting and both VMs, with11
  existing ignored measurements. These heads are now published; remote
  checks and final coordinated delivery remain required.
- Refreshed #22368661980 and #224af4656d7 also passed their complete native gates,
  sessions12382/61314. Both passed121 core tests and nine auxiliary binaries;
  server counts were826/824, with zero failures and11 existing ignored cases.
  Browser, Clippy, SQLx, formatting and both VMs passed on these exact heads.
- #263 at4b43acf8 releases denied export authority before pool cleanup. The
  deterministic regression failed before the fix;43 bounded-export tests passed
  on its own branch and54 on the diagnostic composition. Nixbot382 was running
  at the last check. The PR remains draft.
- [#283](https://github.com/numtide/horae/pull/283), `fix/csv-upload-framing`,
  at8d83b853, is a separate draft over #282. One Content-Length header preserves
  streamed CSV framing;63 test lines cover immediate denials and byte integrity.
  Diagnostic CLI tests and ten original endpoint runs passed. Exact-branch
  verification80585 also passed:20 CLI tests and the real authorization endpoint.
  Remote build352 failed its ARM OIDC boot readiness check; Horae was already
  listening, but the guest control shell missed the900-second deadline. This
  is distinct from the corrected CSV transport failure. Integrate #282 before
  #283; neither the boot candidate nor #283 has been propagated to extraction
  branches yet.
- [#284](https://github.com/numtide/horae/pull/284), `test/vm-store-image`,
  at93b3fbaa, is a ten-line draft over #283. Both complete native VM checks and
  full local flake check passed, including explicit Nix store registration.
  Nixbot402 completed successfully at2026-10-07 23:53:50 UTC on this exact head.
  Both ARM VMs passed: OIDC444.68s and deployment/recovery993.95s test runtime.
  Remote x86 VM checks also passed; this does not verify downstream heads.
- [#285](https://github.com/numtide/horae/pull/285), `test/nix-test-parallelism`,
  at15d9ab5b, makes both Rust runners respect NIX_BUILD_CORES. A sandbox probe
  measured2 allocated cores versus32 default Rust test threads. All121 core
  tests and application suite57770 passed (821 server tests,11 existing ignored
  measurements, plus all auxiliary binaries). Full native flake check79988
  passed on the published head. No Rust assertion or timeout changed, and the CSV failure class is
  not yet considered resolved.
  Nixbot404 subsequently passed ARM tests and formatting on this exact head:
  121 core tests,821 server tests and all nine auxiliary binaries passed with
  an8-thread allocation. The complete exact-head Nixbot404 subsequently passed
  at2026-10-08 00:06:55 UTC, including the remaining VM checks. This is not
  acceptance of downstream extraction heads or every earlier CSV failure.
- [#286](https://github.com/numtide/horae/pull/286), `test/import-cancellation-release`,
  ated286e6b, reuses the CSV session-release observer for the cancelled HTTP-worker
  and page-consumer tests. No production code, SQL, timeout or assertion changes.
  Native full check91786 passed:121 core and821 server tests,11 existing ignored
  measurements, nine auxiliary binaries, browser, Clippy, SQLx and both VMs.
  Remote Nixbot413 passed completely at2026-10-08 01:07:21 UTC on this exact head.
  ARM deployment/recovery947.88s and OIDC397.54s passed; the aggregate covers
  both Linux architectures. The complete native composition8fc3a44e
  passed gate85532; individual refreshed-head verification is still required.
  Exact-head ARM tests on413 now passed:821 server tests,11 existing ignored
  measurements and nine auxiliary binaries, including both corrected cases.
  ARM package, Clippy, SQLx and formatting are now included in the complete pass.
  Earlier partial-state observations are superseded by exact-head aggregate
  acceptance, not used independently to infer completion.
  It is a draft over #285. The shared prerequisite order is now
  #282→#283→#284→#285→#286. The refreshed75-branch tree-only preview includes
  #286 and preserves all57 code-PR review patches. All75 local rebases now match
  their expected trees and all57 review patches retain their edit content;
  all75 branches were published atomically after shared-base acceptance, and
  the11 extraction-root bases were changed to #286. New-head CI remains required.
- Local complete compositiond5c34851 contains both corrections without diagnostic
  logging. Its full gate50229 failed at Clippy: three unused preflight symbols.
  Local compositionf90f60f7 contains #246's function-local lint expectation for this
  deliberately unexposed reader; full gate38699 completed successfully, including
  Clippy, browser, SQLx, tests and both NixOS deployment checks. This is local
  x86 evidence, not ARM or fresh individual-head acceptance. #2469cc4330e passed
  exact-head native all-target Clippy. #264–#268 now inherit the denied-export
  cleanup. #264's111 focused report tests and its actual HTTP session/organization
  matrix passed. Remaining shared-CI rebases and
  final prerequisite integration are still required.
- Updated full composition604c0481 includes #283 framing, the #263 denied-export
  rollback and #246 lint correction, plus #284 VM stores and #285 runner limits.
  Complete native gate14504 passed:191 core tests,1478 server tests,15 auxiliary
  binaries and all other compatible checks, including deployment/recovery91.22s
  and OIDC29.07s. This preserves f90f60f7 as earlier evidence; neither result is
  an ARM VM pass or verification of yet-unpublished extraction heads.
- Complete composition8fc3a44e adds #286's test synchronization to604c0481.
  Native gate85532 exited0:1478 server tests, zero failures,11 existing ignored
  measurements,15 auxiliary binaries, browser, Clippy, SQLx and both VMs passed.
  OIDC took28.24s and deployment/recovery67.24s. Unchanged derivations were reused
  where applicable; ARM/Darwin were explicitly omitted. The exact-head log and
  exit receipt are .scratch/permission-delivery-ci-8fc3a44e-full-check.log and
  .scratch/permission-delivery-ci-8fc3a44e-acceptance.json.

## Remote heads snapshot — 2026-10-07, before the follow-ups above

Shared CI prerequisite, distinct from the58 original-work extractions:
[#282](https://github.com/numtide/horae/pull/282), `test/import-recovery-checks`
at `e39f033a`, is a draft over master containing only the two existing recovery
test files. Current-head Nixbot317 and GitHub Actions37676457227 passed. It is
not another feature extraction. The
CSV fixture patch is now inherited by #224 from this base, not duplicated in its
review delta. The two VM follow-ups remain in #270 pending its dependency refresh.

This earlier snapshot superseded the commit/status labels in the historical
delivery table below; subsequent head changes are recorded above and in the log.
At that snapshot all59 delivery PRs were drafts and remote heads matched local
branches. It had14 successful heads,24 failed heads,14 running
builds and7 new heads without a build aggregate yet. A success
is not completion of the cross-stack review; a failure is not necessarily a new
functional defect. Failure classes and exact-head local evidence are recorded
in the iteration sections. No PR is approved for merging by this inventory.

| PR | Current head | Nixbot aggregate |
| --- | --- | --- |
| #219 | a936c129 | Awaiting fresh build result |
| #220 | c2f587d6 | Awaiting fresh build result |
| #221 | cc474e87 | Running |
| #222 | 1fef1a24 | Running |
| #223 | bc74af8d | Awaiting fresh build result |
| #224 | c374710a | Passed |
| #225 | 34a32928 | Running |
| #226 | 1f9d20e8 | Running |
| #227 | 8f3a7e64 | Running |
| #228 | 951baf9d | Running |
| #231 | 110b365a | Running |
| #232 | c90aa898 | Running |
| #233 | 26bbda2f | Running |
| #234 | 6b524f70 | Running |
| #235 | 7331d30b | Running |
| #236 | 5f0c619c | Running |
| #237 | 7c7d9d6c | Failed |
| #238 | 24aca30d | Running |
| #239 | f4cea603 | Running |
| #240 | 1066cf1a | Awaiting fresh build result |
| #241 | f6b86759 | Awaiting fresh build result |
| #242 | 893219a9 | Awaiting fresh build result |
| #243 | dde4a8a9 | Failed |
| #244 | 7972e987 | Awaiting fresh build result |
| #245 | 7dc76ed3 | Failed |
| #246 | af7bc6fd | Failed |
| #247 | 4c6d0110 | Passed |
| #248 | b6e13979 | Failed |
| #249 | c808b46c | Failed |
| #250 | 202532b8 | Failed |
| #253 | 80058039 | Failed |
| #254 | b9b47c09 | Failed |
| #255 | e16978a9 | Failed |
| #256 | 5a95766b | Failed |
| #257 | 657bf8ed | Passed |
| #258 | e1525af4 | Passed |
| #259 | b94f2fb5 | Failed |
| #260 | f00d8f08 | Failed |
| #261 | 46c0b1c9 | Passed |
| #262 | 91832917 | Passed |
| #263 | 15bda5cc | Failed |
| #264 | 6747c051 | Failed |
| #265 | cec16d16 | Failed |
| #266 | ae53be44 | Failed |
| #267 | 79798acf | Failed |
| #268 | 1be11e74 | Failed |
| #269 | 53183702 | Failed |
| #270 | b480f9d2 | Passed |
| #271 | 5beac2e1 | Passed |
| #272 | 8b8b0c0b | Passed |
| #273 | 65c4aa23 | Passed |
| #275 | f8c1477b | Failed |
| #276 | 7e883eb5 | Passed |
| #277 | d1ab5224 | Failed |
| #278 | 812a870f | Passed |
| #279 | 9fcc0a09 | Failed |
| #280 | 24c5d0e1 | Failed |
| #281 | cd8d1db7 | Passed |
| #282 | e39f033a | Passed |

#219/#220/#223/#224/#225/#227/#231/#238/#239/#240/#242 now depend only on shared CI #282.
Direct-base ancestry is stale for four PRs:
#272 lacks the newer #270 cache/ARM test changes;
#273/#281 lack #272's latest cache deletions; #275 lacks #273's latest cache
deletion. This is not an exhaustive transitive-base audit: named integration
bases also need their own input-head reconciliation. Do not treat green children
as containing newer parent repairs. Remote master and origin/master both resolve
to8b3cc257. Keep #208/#212/#217 outside this refresh.

Evidence: `.scratch/permission-delivery-current.json` (single GitHub snapshot)
and `.scratch/permission-delivery-audit.json` (exact refs, URLs and ancestry).
Refresh only after new results or changes, not on a polling loop.

## Extraction order, scopes and historical verification

The scope/dependency descriptions below retain the delivery history. Their
older head labels and check receipts certify only those named commits, not the
current heads above. Later iteration entries record subsequent verification.

| Delivery | Branch/worktree | Base | Status / acceptance |
| --- | --- | --- | --- |
| Separation ledger | `docs/permission-pr-separation`, `.worktrees/permission-pr-separation` | `02f7b58` | Inventory recorded; no completed-extraction claim |
| Legacy report/invoice readers from #217, [#220](https://github.com/numtide/horae/pull/220) | `fix/report-reader-authority-master`, `.worktrees/report-reader-authority-master` | `02f7b58` | Draft at `bc0c7a0`; 1,127 tests passed, 11 existing ignored; SQLx, offline server/WASM lint, formatting and GitHub Flake Check passed; Nixbot build pending |
| Pure record scopes and grant catalog, [#219](https://github.com/numtide/horae/pull/219) | `refactor/permission-domain-foundation`, `.worktrees/permission-domain-foundation` | `02f7b58` | Draft at `ec7ddbd`; 158 core tests, core Clippy, formatting and GitHub Flake Check passed; Nixbot build pending; no runtime integration |
| Pure rate/management/approval rules, [#221](https://github.com/numtide/horae/pull/221) | `refactor/permission-domain-gates`, `.worktrees/permission-domain-gates` | #219 `ec7ddbd` | Draft at `539316c`; 187 core tests, core Clippy, formatting and full local Flake Check passed; CI required after retargeting |
| Non-activating permission storage, [#222](https://github.com/numtide/horae/pull/222) | `refactor/permission-storage-foundation`, `.worktrees/permission-storage-foundation` | #219 `ec7ddbd` | Draft at `516b023`; original README ICU/migration note restored; 1,170 source-head tests, SQLx and offline lints passed; final documentation-head format and full local Flake Check `69471` passed; remote checks pending |
| Legacy import report conversion lock order, [#223](https://github.com/numtide/horae/pull/223) | `fix/import-report-lock-order`, `.worktrees/import-report-lock-order` | `02f7b58` | Draft at `c8f95ac`; 1,125 tests passed, 11 existing ignored; SQLx, offline server/WASM lint format and current-head GitHub Flake Check passed; Nixbot pending |
| Interrupted import session cleanup, [#224](https://github.com/numtide/horae/pull/224) | `fix/import-session-cleanup`, `.worktrees/import-session-cleanup` | master `8b3cc257` | Draft at `2afe25e3`; original cleanup repair retained, plus bounded synchronization of two aborted-CSV test paths.55 engine tests and format passed on the correction; sole full native71277 running. Earlier checks do not certify this new head |
| Current authority for organization branding writes, [#225](https://github.com/numtide/horae/pull/225) | `fix/branding-current-authority`, `.worktrees/branding-current-authority` | `02f7b58` | Draft at `f2d6bd4`; full suite, SQLx, offline server/WASM lint format and current-head GitHub Flake Check passed; Nixbot pending |
| Internal reusable-profile commands, [#226](https://github.com/numtide/horae/pull/226) | `refactor/permission-template-commands`, `.worktrees/permission-template-commands` | #222 `e9695fd` | Draft at `82d15f3`; 1,189 tests, SQLx, offline server/WASM Clippy, format and full local Flake Check passed; CI required after retargeting; no endpoints or activation |
| Current authority for user creation/role/activity, [#227](https://github.com/numtide/horae/pull/227) | `fix/user-mutation-authority`, `.worktrees/user-mutation-authority` | `02f7b58` | Draft at `142eda1`; 1,127 tests, SQLx, offline server/WASM Clippy, format and current-head GitHub Flake Check passed; Nixbot build pending |
| Assignment authority and project writer coordination, [#228](https://github.com/numtide/horae/pull/228) | `fix/project-access-lock-order`, `.worktrees/project-access-lock-order` | #227 `142eda1` | Draft at `0e1e675`; 1,137 tests, SQLx, offline server/WASM Clippy, format and full local Flake Check passed; master-targeted CI required after retargeting |
| Durable CSV preparation outside SQL transactions, [#231](https://github.com/numtide/horae/pull/231) | `fix/csv-batch-transaction-boundary`, `.worktrees/csv-batch-transaction-boundary` | `02f7b58` | Draft at `e9898ed`; 1,122 tests, SQLx, offline server/WASM Clippy, format and current-head GitHub Flake Check passed; Nixbot build pending |
| Financial snapshot reader authority, [#232](https://github.com/numtide/horae/pull/232) | `fix/financial-snapshot-authority`, `.worktrees/financial-snapshot-authority` | #220 `bc0c7a0` | Draft at `0bb5721`; 1,141 tests, SQLx, offline server/WASM Clippy, format and full local Flake Check passed; required CI after retargeting |
| Invoice writer/revocation ordering, [#233](https://github.com/numtide/horae/pull/233) | `fix/invoice-write-authority`, `.worktrees/invoice-write-authority` | Integration base `0046dad` combining #227/#228 and #220/#232 | Draft at `8a6cb2a`; 1,162 tests, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget to master after prerequisites, do not merge into integration base |
| Internal person-profile commands, [#234](https://github.com/numtide/horae/pull/234) | `refactor/person-profile-commands`, `.worktrees/person-profile-commands` | Integration base `46f02f7` combining #226/#221 | Draft at `45d219e`; 1,245 tests, schema upgrade, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget to master after prerequisites, no activation |
| Harvest connection change authority, [#235](https://github.com/numtide/horae/pull/235) | `fix/harvest-connection-authority`, `.worktrees/harvest-connection-authority` | #228 `0e1e675` | Draft at `2fcecd1`; 1,146 tests, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget after #227/#228 |
| Import job command and download authority, [#236](https://github.com/numtide/horae/pull/236) | `fix/import-job-authority`, `.worktrees/import-job-authority` | Integration base `26d6159` combining #235/#231 | Draft at `95bdf4a`; 1,163 tests, SQLx, offline server/WASM Clippy, format and full local Nix passed; retarget to master after #227/#228/#235 and #231, not an integration-base merge |
| Original import requester provenance, [#237](https://github.com/numtide/horae/pull/237) | `feat/import-job-requester`, `.worktrees/import-job-requester` | Integration base `cfb8240` combining #236/#219/#222 | Draft at `2242361`; 1,218 tests, schema upgrade, SQLx, offline server/WASM Clippy and format passed; full Nix passed on unchanged diagnostic rerun; initial inherited-menu failure retained; retarget after both prerequisite chains, no worker-policy activation |
| Budget email preparation authority, [#238](https://github.com/numtide/horae/pull/238) | `fix/budget-email-authority`, `.worktrees/budget-email-authority` | Master `1b8fa4f` | Draft at `7a7cede`; 1,133 source-head tests, SQLx, format and cache-inclusive server/WASM Clippy passed; full current-head local Nix passed; no real mail or policy activation |
| Approval tenant isolation, [#239](https://github.com/numtide/horae/pull/239) | `fix/approval-tenant-isolation`, `.worktrees/approval-tenant-isolation` | Master `1b8fa4f` | Draft at `66a256f`; three source/test files and complete cache patch preserved exactly; 1,124 source-head tests, source review, SQLx, format and offline server/WASM Clippy passed; full current-head local Nix and required GitHub checks passed; initial remote browser failure and unchanged rerun retained |
| Identity response projections, [#240](https://github.com/numtide/horae/pull/240) | `fix/identity-response-projections`, `.worktrees/identity-response-projections` | Master `1b8fa4f` | Draft at `a19ea63`; source suite/cache/lints, corrected Clients fixture, complete local Nix `99785` and GitHub Flake Check passed; original failures retained; remote Nixbot failures remain unresolved; no activation |
| Time-writer account activity, [#241](https://github.com/numtide/horae/pull/241) | `fix/time-write-activity`, `.worktrees/time-write-activity` | #228 `0e1e675` | Draft at `7820f8d`; 1,142 tests, cache provenance, bounded review, format, offline server/WASM lint and full Nix `85167` passed; prerequisite integration/retarget/current-head CI still required; no delegated writes or activation |
| Time-entry invoice identity boundary, [#242](https://github.com/numtide/horae/pull/242) | `fix/time-entry-payload`, `.worktrees/time-entry-payload` | Master `1b8fa4f` | Draft at `43337fc`; 1,122 tests, cache/source provenance, bounded review, format, offline server/WASM lint and full Nix `73808` passed; required GitHub checks passed on unchanged rerun; initial cancellation-test failure retained; delivery review and Nixbot diagnosis remain; no policy or UI change |
| Session-bound project-manager delegation, [#243](https://github.com/numtide/horae/pull/243) | `feat/project-manager-delegation`, `.worktrees/project-manager-delegation` | Review base `e44433e` combining #234/#228 | Draft at `3404c85`; suite/cache, final WASM lint and full local Nix passed (cached result confirmed in `10972`); final server gate closed; no form wiring or activation |
| Own-permission explanation and Settings, [#244](https://github.com/numtide/horae/pull/244) | `feat/own-permission-settings`, `.worktrees/own-permission-settings` | #234 `45d219e` | Draft at `6c4e4d1`; 1,273 Rust tests, SQLx, provenance, review/Spec Kit/format/detector, offline server/WASM lint and isolated Chromium passed; desktop/mobile captures inspected; full local Nix passed (cached result confirmed in `30710`); no activation |
| Permission audit history, [#245](https://github.com/numtide/horae/pull/245) | `feat/permission-audit-history`, `.worktrees/permission-audit-history` | Review base `59d2798` combining #243 `3404c85` and #244 `6c4e4d1` | Draft at `14ad9ca`; source suite/cache/lints, corrected session-actor fixture and complete final local Nix `56923` passed, including own/history browser suites; original failures retained; remote Nixbot failures, prerequisite integration and retargeted gates remain open |
| Read-only legacy permission diagnostics, [#246](https://github.com/numtide/horae/pull/246) | `feat/permission-preflight`, `.worktrees/permission-preflight` | Review base `61c90bc` combining #237 `2242361` and #226 `82d15f3` | Draft at `ff482c8`; suite/cache/lints passed; full Nix `99711` failed on inherited editor loading timeout; unchanged focused transport/editor sequence and full unchanged-head rerun `41382` passed; initial root cause unproven; no endpoint, UI, migration or activation |
| Materialized XLSX/PDF authorization, [#247](https://github.com/numtide/horae/pull/247) | `fix/materialized-export-authority`, `.worktrees/materialized-export-authority` | Review base `3edc0b8` combining existing `0046dad` (#227/#228 + #220/#232) and #222 `e9695fd` | Draft at `d9717e7`; 1,229 tests, full SQLx/provenance (1,135 descriptors), source review/Spec Kit/format, offline native/WASM lint and complete local Nix `32625` passed; remote Nixbot failures, prerequisite integration and retargeted gates remain open; no policy activation |
| CSV delivery authorization, [#249](https://github.com/numtide/horae/pull/249) | `fix/csv-export-authority`, `.worktrees/csv-export-authority` | Exact #247 head `d9717e7` | Draft at `71232dc`; 1,245 source-head tests, full SQLx/provenance (1,174 descriptors), format/source review/scoped analysis and final-head offline native/WASM lint passed; complete local Nix `72947` passed; retargeted required checks remain; no canonical activation |
| Existing permission specification and history, [#248](https://github.com/numtide/horae/pull/248) | `docs/permission-specification`, `.worktrees/permission-specification` | Master `1b8fa4f` | Draft at `49843b2`; all 54 original feature documents preserved, six contextualized; all 43 requirements/criteria and 236 task lines unchanged; original New Project transition and AGENTS cache guidance preserved; provenance/format passed, full local Nix `51945` and required GitHub checks passed; final reconciliation pending; no code or constitution adoption |
| Requester-bound editor API, [#250](https://github.com/numtide/horae/pull/250) | `feat/permission-editor-api`, `.worktrees/permission-editor-api` | Review base `0117991` combining merged master `ed558f6` and #245 `14ad9ca` | Draft at `c727bc8`; four owned commits patch-equivalent to preserved `c82a5b3`; full native Nix `46207` PASSED, including complete browser/deployment/OIDC. Old `90520` selector failure retained; corrected helper inherited from merged master. Combined full native Nix `16434` passed on `7a2d61c`; retargeted gates remain; no UI or activation |
| Scoped people directory, [#253](https://github.com/numtide/horae/pull/253) | `feat/scoped-people-directory`, `.worktrees/scoped-people-directory` | Review base `0117991`, independent of #250 editor operations and #240 legacy projections | Draft at `3a37538`; original reader/DTO/endpoint and seven DB/HTTP tests preserved; tests/Clippy/live SQLx `18891` and full local Nix `2803` passed. Exact browser/e2e/OIDC outputs explicitly materialized from signed cache; combined full native Nix `16434` passed on `7a2d61c`; later reader composition and retargeted gates remain; no UI or activation |
| Identity-only project-team choices, [#254](https://github.com/numtide/horae/pull/254) | `feat/project-people-picker`, `.worktrees/project-people-picker` | #253 `3a37538`, for shared `PeopleCursor` and inherited foundations | Draft at `f498c3f`; original reader/DTO/endpoint, nine DB tests and HTTP assertions preserved; full native Nix `81722` PASSED, including complete browser/deployment/OIDC and exact cache reuse where available. Combined full native Nix `16434` passed on `7a2d61c`; retargeted gates remain; no picker UI, assignment writes or activation |
| Scoped time-entry reader, [#255](https://github.com/numtide/horae/pull/255) | `feat/scoped-time-reader`, `.worktrees/scoped-time-reader` | Review base `0117991`, independent of directory, project-team choices and editor operations | Draft at `d93e1af`; original `4ac30fa` DTO/reader/eight DB tests/HTTP assertions and endpoint preserved; 23 original SQLx descriptors, module registrations adapted only. Formatting/provenance passed; full native Nix `40092` PASSED; wider composition and retargeted checks pending; no Timesheet UI, subject discovery, commands or activation |
| Timesheet person discovery, [#256](https://github.com/numtide/horae/pull/256) | `feat/timesheet-people-discovery`, `.worktrees/timesheet-people-discovery` | Review base `40102ae`, combining #255 `d93e1af` admission reader and #253 `3a37538` shared `PeopleCursor` | Draft at `1552fdb`; original `60f60f9` DTO/reader/eight DB tests, endpoint/HTTP additions and five SQLx descriptors preserved. Tests/Clippy/live SQLx `61768` PASSED; application992 passed, zero failed,11 inherited ignored. Full native Nix `37414` PASSED; wider integration pending; no UI, context-page contract, commands or activation |
| Requester-bound Timesheet page context, [#257](https://github.com/numtide/horae/pull/257) | `feat/timesheet-page-context`, `.worktrees/timesheet-page-context` | #256 `1552fdb`, for subject discovery and shared read admission | Draft at `b30e3cd`; DTO/reader original, all six DB tests retained with a four-line cancellation-barrier synchronization correction. Old `8e09e60` full gate passed; dependent `d2b45ca` exposed a test timeout, retained below. Current-head full native Nix `40402` PASSED, including998 application tests, zero failed,11 inherited ignored, browser and NixOS/OIDC; retargeted checks remain |
| Person-bound Timesheet commands, [#258](https://github.com/numtide/horae/pull/258) | `feat/timesheet-person-commands`, `.worktrees/timesheet-person-commands` | #257 `b30e3cd`, for shared context contracts and foundations | Draft at `b0eacfd`; original commands/tests and39 descriptors unchanged. Old `64524` failure retained; corrected-head tests/Clippy/live SQLx `98625` PASSED (1011 application tests, zero failed,11 inherited ignored). Full native Nix `7633` PASSED; wider integration pending; no UI or activation |
| Selected-person Timesheet UI, [#259](https://github.com/numtide/horae/pull/259) | `feat/timesheet-selected-person-ui`, `.worktrees/timesheet-selected-person-ui` | #258 `b0eacfd`, for page context, discovery, tracking and commands | Draft at `4ce0919`; original UI/submission contract/tests/cache plus recovered original Nix asset path and two-line calendar pointer-target CSS. Nine whole source/test files byte-identical to final original. Failed gates `48062`/`91457` retained below; full native Nix `75088` PASSED; published wider combination `1a87961` gate88592 running; no activation or new feature |
| Scoped People and permission editor UI, [#260](https://github.com/numtide/horae/pull/260) | `feat/people-permission-editor-ui`, `.worktrees/people-permission-editor-ui` | Verified review base `7a2d61c`, combining #240/#250/#253/#254 and foundations; actual consumer dependencies are identity, editor API and directory | Draft at `98857cf`; fifteen complete files original, legacy tasks and current Clients retained. Sixteen JS tests, source review, whitespace/provenance pass; full native Nix `66677` PASSED. Published shared-navigation combination `1a87961` gate88592 running; no activation |
| Cross-PR Timesheet/permission verification only | `integration/timesheet-permission-check`, `.worktrees/timesheet-permission-integration` | Combines #240/#241/#250/#253–#262 and inherited foundations | Published at `1a87961`, no delivery PR; all five navigation guards,29 browser suites and both HTTP registries retained.19 navigation/storage tests and formatting passed; full gate88592 PASSED,1098 application tests/zero failed/11 inherited ignored and all remaining checks. Excludes later #263–#266 exports/access and future Reports consumer |
| Cross-PR reader/editor verification only | `integration/permission-readers-editor-check`, `.worktrees/permission-readers-editor-check` | Combines #240 `a19ea63`, #250 `c727bc8`, #253 `3a37538` and #254 `f498c3f` | Published at `7a2d61c`, no delivery PR or merge target; registration conflicts resolved preserving both sides, dedicated source/test blobs unchanged, original combined users module restored exactly. Tests/Clippy/SQLx `24884` and full native Nix `16434` PASSED; exact browser/deployment/OIDC outputs and logs verified after original process terminated. Later #255–#257 not included |
| Scoped detailed time report, [#261](https://github.com/numtide/horae/pull/261) | `feat/scoped-time-report-reader`, `.worktrees/scoped-time-report-reader` | #257 `b30e3cd`, shared time-read admission | Draft at `bf452dd`; final detailed reader,13 original DB tests, reader HTTP/totals assertions and19 original descriptors. Source/provenance/format pass; full native Nix `37229` PASSED; no UI/export/activation |
| Scoped grouped time report, [#262](https://github.com/numtide/horae/pull/262) | `feat/scoped-time-report-groups`, `.worktrees/scoped-time-report-groups` | #261 `bf452dd`, report contracts/totals and fixture | Draft at `66dbf0b`; final reader and13 DB tests exact; original171-line HTTP fixture and10 original descriptors. Source/provenance/format pass; full native Nix `79456` PASSED,1024 application tests/zero failed/11 inherited ignored plus all remaining gates; no UI/export/activation |
| Scoped XLSX time exports, [#263](https://github.com/numtide/horae/pull/263) | `feat/scoped-time-xlsx`, `.worktrees/scoped-time-xlsx` | Review base `e8b95cc`, combining #261 and #247 with their existing foundations | Draft at `8cc11c3`; original cbc78a8 materialization/release checks,11 DB tests and HTTP assertions preserved. Final query predicates retained;11 original cache additions/one obsolete descriptor removed. Full native gate48482 PASSED,1064 application tests/zero failed/11 inherited ignored and remaining checks; all ten derivations previously matched clean head. Wider exports/UI composition remains open |
| Scoped CSV time exports and shared filters, [#264](https://github.com/numtide/horae/pull/264) | `feat/scoped-time-csv`, `.worktrees/scoped-time-csv` | Review base `4e0ed43`, combining #263 and #249 | Draft at `55d362b`; original10 DB tests/five parser tests/two-format HTTP fixture retained; native cursor final predicates, shared XLSX release helpers and17 original descriptors. Source/provenance/format pass; full native gate40286 PASSED:1095 application tests/zero failed/11 inherited ignored plus all remaining checks; grouped exports/UI/policy binding remain separate |
| Report access and permission-mode-bound downloads, [#265](https://github.com/numtide/horae/pull/265) | `feat/time-report-access`, `.worktrees/time-report-access` | #26455d362b scoped downloads and shared parser | Draft at `f436a29`;7266abb backend preflight/DTOs and mode binding, original parser/HTTP assertions unchanged; format and source review pass; full native gate1825 running; no UI or activation |
| Grouped CSV/XLSX time exports, [#266](https://github.com/numtide/horae/pull/266) | `feat/grouped-time-exports`, `.worktrees/grouped-time-exports` | Review base `da493f6` combining #265 and #262 | Draft at `0eec1a4`; original grouped handlers/source/16 DB tests, shared CSV group authorization and18 descriptors; original grouped-filter HTTP fixture retained. Source/format/provenance pass; full native gate40587 running; later URL controls and consumer/browser integration remain open |
| Time download result filters, [#267](https://github.com/numtide/horae/pull/267) | `feat/time-report-download-filters`, `.worktrees/time-report-download-filters` | #2660eec1a4 and inherited detailed/grouped report/export foundations | Draft at `e029a89`; original active-project/billability URL parser, seven DB tests, strict transport and actual-session fixtures;10 original descriptors. Format/provenance/source review passed; full native gate51012 running; UI/browser filter propagation retained separately |
| Retained full-feature follow-up | Original refs and unpublished snapshot; acceptance classification below | Final operation contracts, governance and cutover prerequisites | Not an additional implemented delivery; preserve incomplete work without activating policy or marking broad acceptance tasks complete |
| Scoped Reports screen, [#268](https://github.com/numtide/horae/pull/268) | `feat/scoped-time-report-ui`, `.worktrees/scoped-time-report-ui` | #267, inherited report access/readers and all four export routes | Draft at `1be11e74`; original disclosure glyphs and browser path assertions retained. Full native68739 passed at this head, replacing disk-failed31708. ARM Nixbot297 remains red; wider review/full T203 remain open |
| Cross-PR Reports/Timesheet/People verification only | `integration/reports-permission-check`, `.worktrees/reports-permission-integration` | Combines verified `1a87961` with #268 `810ce57` and its report/export foundations | Local integration merge `b7836d7`, not a delivery PR or GitHub merge. Both HTTP registration sets and all30 unique browser suites retained; dedicated source/test blobs unchanged. Format/syntax/source comparison pass; full native gate25958 running |
| Scoped project editor, [#269](https://github.com/numtide/horae/pull/269) | `feat/scoped-project-editor`, `.worktrees/scoped-project-editor` | `integration/permission-readers-editor-check` | Draft at `53183702`; Nixbot301 SQLx/browser/package/lint checks passed on both Linux architectures. Native tests failed on CSV crash-resume Busy; correction in #224 not propagated here. ARM deployment and earlier intermittent lost-acknowledgement diagnosis remain open; no blind native rerun |
| Scoped project overview/detail, [#270](https://github.com/numtide/horae/pull/270) | `feat/scoped-project-reads`, `.worktrees/scoped-project-reads` | `integration/project-read-prerequisites` | Draft at `0b4fd421`; Nixbot302 passed all x86 checks and ARM package/browser/lint/tests/SQLx at this head. ARM deployment/OIDC still running when inspected; no duplicate native rerun needed. Earlier295 does not prove a root-cause fix for intermittent266 |
| Scoped project CSV/XLSX delivery, [#271](https://github.com/numtide/horae/pull/271) | `feat/scoped-project-exports`, `.worktrees/scoped-project-exports` | `integration/project-delivery-prerequisites` at375e9bd, combining #270 and #267 | Draft at `5beac2e1`; original requester HTTP matrix restored and registered. Nixbot296 passed all listed checks on both Linux architectures at this head; no repeat of disk-failed37940 needed. Wider integration review pending |
| Scoped Harvest-compatible project reads, [#272](https://github.com/numtide/horae/pull/272) | `feat/scoped-harvest-projects`, `.worktrees/scoped-harvest-projects` | #270 | Draft at `8b8b0c0b`; Nixbot303 passed all x86 checks and ARM package/browser/lint/tests/SQLx at this head; no duplicate native rerun needed. Latest parent cache removals, ARM deployment/OIDC and wider integration remain pending |
| Scoped task catalog/tracking reads, [#273](https://github.com/numtide/horae/pull/273) | `feat/scoped-task-reads`, `.worktrees/scoped-task-reads` | #272 | Draft at `65c4aa23`; Nixbot304 passed all x86 checks and ARM package/browser/lint/tests/SQLx/OIDC at this head; no duplicate native rerun needed. Latest parent cache removals, ARM deployment and wider integration remain pending |
| Current task creation and explicit rate edits, [#275](https://github.com/numtide/horae/pull/275) | `feat/scoped-task-writes`, `.worktrees/scoped-task-writes` | #2734b2c87d | Draft at `f8c1477`; tests/live-schema SQLx96425 and full native71588 passed. ARM/wider acceptance pending. No lifecycle, catalog UI, later atomic rate creation or activation |
| Task archive/restore and import preservation, [#276](https://github.com/numtide/horae/pull/276) | `feat/scoped-task-lifecycle`, `.worktrees/scoped-task-lifecycle` | integration/task-lifecycle-prerequisites1955c38 (#275/#269/#258/#223/#224/#231) | Draft at `7e883eb`; tests/live-schema SQLx60667 and full native95578 passed after restoring original descriptor2889c08. Base full native65768 passed. ARM/wider acceptance pending; catalog/link/UI controls excluded |
| Existing project-task link authority and currency, [#277](https://github.com/numtide/horae/pull/277) | `feat/scoped-task-links`, `.worktrees/scoped-task-links` | #2767e883eb | Draft at `d1ab522`; original979a594 extracted. Tests/live-schema SQLx23598 and full native62455 passed. ARM/wider acceptance pending; no UI or activation |
| Atomic task creation with an initial rate, [#278](https://github.com/numtide/horae/pull/278) | `feat/atomic-task-creation`, `.worktrees/atomic-task-creation` | #277d1ab522 | Draft at `812a870`; full native63014 passed, including browser and deployment/OIDC. ARM/final cross-PR review pending. Catalog UI separate |
| Task catalog and editor, [#279](https://github.com/numtide/horae/pull/279) | `feat/task-catalog`, `.worktrees/task-catalog` | integration/task-catalog-prerequisitese9f793d (#278/#260) | Draft, published `9fcc0a0`; full native66969 passed on this head, using valid cached outputs for identical derivations. ARM/final cross-PR review pending |
| Task catalog dependency verification only | `integration/task-catalog-prerequisites`, `.worktrees/task-catalog-prerequisites` | #278812a870 and #260f00d8f0 | Published `e9f793d`; original requester,29 browser suites/20 HTTP matrices retained; source audit and full native31756 passed, including deployment/OIDC. Not a delivery PR or wider cross-stack acceptance |
| Project task archive/restore controls, [#280](https://github.com/numtide/horae/pull/280) | `feat/project-task-activity-ui`, `.worktrees/project-task-activity-ui` | #276 at `7e883eb` | Draft at `24c5d0e`; four original8c1bf9b paths, parent receipt cleanup retained. Full native14092 passed, including browser and deployment/OIDC. ARM/final cross-PR review pending |
| Harvest-compatible client reads, [#281](https://github.com/numtide/horae/pull/281) | `feat/scoped-harvest-clients`, `.worktrees/scoped-harvest-clients` | #272 at `fd91c3d` | Published draft `cd8d1db7` extracts eleven dirty paths plus shared-test visibility and original fixture descriptor81aefb2e. Initial failures28333/65280/11982 corrected; full native15935 and Nixbot293 on both Linux architectures passed at this head. Latest parent cache changes and wider review remain pending; T238 not implemented |

The original candidate groups have now produced58 extraction PRs, separate
from five shared CI corrections (#282–#286) and this ledger (#218). Remote
inventory aafeac confirmed the preceding62 extraction/CI PRs as open drafts;
new draft #286 brings that count to63. These are ownership counts, not readiness counts. Integration
branches are review/verification bases, not additional deliveries or merge
targets; retarget only after their real prerequisites land. Current-head CI,
final review and explicit preservation of unfinished acceptance remain required.

### First extraction: legacy report and invoice reader authority

Purpose: reauthorize `report_detailed`, `list_invoices` and `get_invoice`
after the initial session lookup, and keep invoice headers/lines in one
snapshot. Preserve existing Manager/Admin policy, result sizes, filters and
errors; introduce neither canonical grants nor UI changes.

Dependencies verified by source inspection:

- `snapshot::manager` from `22ffdab`: bounded REPEATABLE READ authorization,
  organization/actor SHARE locks, retry and timeout restoration.
- `db::OrganizationLock`/`lock_organization` from `3ae8e08`; absent from
  #216. Do not cherry-pick the entire project-access commit merely for this helper.
- Existing base already supplies executor-based report reads,
  connection-based invoice projection, test seed/blocking helpers and the
  registered-session HTTP harness.
- Preserve #216's `fetch_invoices` query with its optional client filter and
  stable `created_at, id` ordering. Wrap it rather than copying #217's older
  list query.
- Bring only the relevant HTTP test additions and exact endpoint matching;
  do not copy the parent's unrelated authorization test modules.
- Include all helper/query SQLx descriptors and regenerate/verify the extracted
  cache, not only the eight descriptors in #217's own commit.

Acceptance scope on the new head: all seven reader regressions, authenticated
HTTP cases, tenant/filter/not-found behavior, direct and gated revocation in
both orders, coherent invoice data, cancellation, inherited settings, #216
client-invoice regressions, cross-writer lock review, full server/core tests,
offline SQLx compilation, server/WASM lint, formatting and required Nix gates.
All local checks above passed as recorded below; required CI/Nix gates remain
pending. Historical tests on `5faed76` do not prove this extraction.

### Shared foundations and migration constraints

- First domain delivery contains `AccessScope`, the grant catalog, built-in
  selections and strict restoration of saved selections. Its five new files
  match `524c29e` exactly; the only adaptation is adding `pub mod permissions`
  to the current core root while preserving #216's `client` module.
  This accounts for the code/test hunks of `a7727f1`, `e8dcb77` and `524c29e`;
  their specification hunks remain preserved for specification reconciliation.
  The later profile-name validator from `ec35446` stays with profile storage;
  person-management, rates and approval coverage are separate dependants.
- Pure scope/catalog/rate/approval-record rules can be evaluated separately
  from runtime policy activation.
- Storage and command receipts/assignment models precede canonical consumers.
  Editor, People, Timesheet, Reports, project and task consumers must keep
  policy-zero behavior until the separately approved full cutover.
- `0042` adds policy version defaulting to zero, storage and
  `users(org_id,id)` uniqueness; it performs no legacy mapping.
- `0043` receipts and `0045` import requester provenance depend on that
  compound user key. Provenance is not a clean standalone cherry-pick as written.
- `0044` keeps management separate from tracking; `0047` name collation
  depends on `0042` template storage.
- `0048` task lifecycle/view logic depends on `0042` policy version and must
  stay with corresponding task commands/readers. Its backfill is canonical-only.
- `0046` supplies the native CSV cursor function; review source/release
  authorization and streaming consumers together before extracting it.
- Preserve migration identities/checksums unless a necessary adaptation is
  explicitly documented and verified on disposable databases. Do not run
  migrations against existing user data.
- The sole Nix check change makes crate JS assets visible to browser scripts.
  Keep it with tests that require those assets. #212 changes no Cargo dependency
  files or CI workflow.
- Shared legacy authority/lock and export repairs need cross-command review;
  do not declare them independent from titles alone.

## Spec Kit reconciliation

The existing `015-scoped-permissions` spec/plan/tasks were resolved using
`check-prerequisites.sh --json --require-tasks --include-tasks`, with no
feature-directory override or file changes. No Analyze hooks were present.
This was a separation-focused partial consistency analysis, not a clean
full-feature acceptance report.

The working-copy task inventory has 238 unique IDs: 208 checked and 30 unchecked.
These are bookkeeping counts, not a percentage of application completion.
All 36 FRs have candidate task or section-context references; all nine success
criteria still require full acceptance. Do not infer coverage from task-title
regexes alone.

Unchecked tasks: T237, T238, T230, T234, T203, T042, T006, T007, T008, T009, T010, T011, T037, T038, T012, T013, T043, T044, T045, T046, T014, T015, T039, T040, T016, T017, T041, T018, T019, T020.

Evidence reconciliation:

- T161's earlier prose says pending, but `1b81680` explicitly closes the
  checkbox and reconciles `2497dbe` to frozen-source verification
  (repeat `52467`, 1,751 tests, browser/lint/SQLx/VM gates). Preserve that
  historical completion; clarify the stale paragraph when carrying the docs.
  It proves neither later heads nor the extracted branch.
- The dependent-spec register's #216 reference `5a459c4` predates its
  authority and browser corrections. The integration base is the actual
  `02f7b58` merge above.
- The source contains constitution 1.1.0, while master uses 1.0.0. Preserve the
  existing amendment as its own governance/specification material; it is not
  a prerequisite for a legacy-reader security repair.
- Full approval/withdrawal, company locks, migration/cutover, financial report
  and unresolved operation contracts remain unfinished. Do not finish them
  as part of splitting or silently turn their drafts into accepted behavior.

### Retained acceptance versus extracted implementation

This classification covers all30 unchecked source task IDs without changing
their original checkboxes. It is a separation inventory, not a new feature plan
or a claim that the remaining work is fully specified. The committed feature
documents remain in #248; the seven unpublished Clients documents and the
unadopted constitution proposal retain their separate preservation described
above. Existing implementation is delivered by its owning PR, not duplicated
into a new follow-up PR merely because an umbrella task stays unchecked.

| Source task IDs | Extracted or preserved scope | What remains outside completed-extraction claims |
| --- | --- | --- |
| T237 | Harvest-compatible client reads in #281 | Source checkbox remains unchecked; extraction checks and current-head readiness are recorded separately. Does not imply ordinary Clients workflow or policy activation |
| T238 | Approved FR-035/036 retained in #281's contract and original unpublished documents | Ordinary Clients reads/UI/writes, protected default-rate intent, archive/restore races and workflow selector reconciliation remain incomplete |
| T230, T234 | Task reads, commands, catalog and project controls in #273/#275–#280 | Full task-management acceptance and unresolved scope must be reconciled against those deliveries; do not describe the existing catalog UI as missing or silently close the umbrella tasks |
| T203 | Report readers, exports and scoped consumer in #261–#268 | Full candidate discovery/pickers, financial report families and broad Reports acceptance remain open; existing grouping and result filters are already extracted |
| T006, T007, T008, T009, T042 | Existing research, matrix, hierarchy and migration/governance proposals retained in #248/original refs | Final operation contracts, role-check inventory, constitution ratification, full lock hierarchy and executable remaining-work refinement are not supplied by this split |
| T010, T011, T016, T017, T018, T037, T038, T041 | Domain/storage/profile commands, editor APIs/UI and audit increments in #219/#221/#222/#226/#234/#243–#245/#250/#260 | Broad profile/application/audit/Workspace acceptance remains unchecked despite implemented sub-increments. No policy replacement or broad task closure is inferred from their tests |
| T012, T013, T043, T044, T045, T046 | Pure coverage rules in #221 and legacy tenant isolation in #239; remaining contracts preserved | Full scoped approvals/withdrawal, combined expense visibility, company-lock calculation/storage/scheduling and UI are not implemented by these narrower deliveries |
| T014, T015, T019, T020, T039, T040 | Existing per-surface authorization, transaction and regression increments have their own extraction owners | Complete financial payload coverage, every entry point/writer, migration/identity mapping, policy cutover and all nine success criteria remain full-feature work, not prerequisites to invent during separation |

No new product decisions, grants or implementation commitments are introduced
by this classification. Original task text and its supporting contracts remain
authoritative; the split can finish with unfinished work explicitly retained,
but cannot finish with required extraction verification still missing.

## #208 overlap — read-only

#208 remains outside this work. Its `48a4156` branch shares 38 changed paths
with #212, seven of them SQLx descriptors. Path overlap is not proof of a text
conflict. Recheck integration later without modifying #208.

Non-cache overlaps:

- `.specify/feature.json`
- `crates/core/src/budget.rs`
- `crates/core/src/lib.rs`
- `crates/horae/assets/css/horae.css`
- `crates/horae/src/components/sidebar.rs`
- `crates/horae/src/main.rs`
- `crates/horae/src/models/assignment.rs`
- `crates/horae/src/models/project.rs`
- `crates/horae/src/pages/new_project.rs`
- `crates/horae/src/pages/projects.rs`
- `crates/horae/src/pages/projects/fee_balances.rs`
- `crates/horae/src/pages/reports.rs`
- `crates/horae/src/reports.rs`
- `crates/horae/src/reports/limits.rs`
- `crates/horae/src/reports/streaming.rs`
- `crates/horae/src/reports/streaming/database_tests.rs`
- `crates/horae/src/route.rs`
- `crates/horae/src/server_fns.rs`
- `crates/horae/src/server_fns/budgets.rs`
- `crates/horae/src/server_fns/invoices/tests.rs`
- `crates/horae/src/server_fns/invoices/tests/imported_rates.rs`
- `crates/horae/src/server_fns/organization.rs`
- `crates/horae/src/server_fns/projects.rs`
- `crates/horae/src/server_fns/reports.rs`
- `crates/horae/tests/browser/action-errors.cjs`
- `crates/horae/tests/browser/new-project-permissions.cjs`
- `crates/horae/tests/browser/new-project.cjs`
- `crates/horae/tests/browser/project-task-rates.cjs`
- `crates/horae/tests/browser/run-design-checks.sh`
- `crates/horae/tests/detail_navigation.rs`
- `crates/horae/tests/new_project_screen.rs`

## Original commit accounting

Every original commit is retained in the verified bundle and refs. Entries
below identify delivery ownership; ownership does not imply current-head CI
acceptance. Verification receipts embedded in these rows are historical unless
explicitly reconciled in the current verification follow-ups. Mixed commits
may supply multiple deliveries with the documented extraction adaptations.

Documentation ownership now applies across all mixed commits as well as the 50
documentation-only rows: final committed `specs/015-scoped-permissions/` content
is preserved in #248. All intermediate versions remain in the original refs.
Original `specs/011-new-project-screen/spec.md` permission-transition hunks also
belong to #248, composed over master's newer expense requirements. The original
constitution 1.1.0 proposal remains unadopted work in #212 and its backup ref;
the obsolete feature-selector change is retained historically, not reapplied
over feature 016. Original README migration prerequisites are restored in #222
(`516b023`), and AGENTS cache guidance is preserved exactly in #248 (`49843b2`). Unpublished
Clients documents are separate and are not silently included in #248.

| Source | Original change | Candidate responsibility | Disposition |
| --- | --- | --- | --- |
| `05448a8` | Specify scoped roles and permissions | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance; obsolete feature-selector change retained historically, not reapplied over feature 016 |
| `1d45191` | Require Harvest parity for permissions and scoped approvals | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `a7727f1` | Add record scope evaluation for permissions | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `2abce9a` | Enforce approval isolation and record permission boundaries | approval-isolation | Three Rust/test files and original cache patch in independent #239 (`66a256f`); 1,124 tests, source review, SQLx, format and offline server/WASM lints passed; full local Nix passed; mixed specification hunks retained |
| `757f43d` | Enforce tenant and administrator boundaries for assignments | legacy-access-writers | Rust/test/cache changes in #228 with subsequent coordination repair; specification hunks retained |
| `d3a4ff3` | Document profile reapplication and import permission boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `dcf21ef` | Specify permission migration safeguards and rate-scope verification | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `b3ee8da` | Align authorization governance with scoped permission profiles | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance; constitution 1.1.0 remains a separately preserved, unadopted proposal in original #212 |
| `b569c75` | Extend permission specification to confirmed web domains | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `f5cf02d` | Record expense scope defaults and lifecycle permission gaps | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6ce9071` | Document expense action scope and independent lock states | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `b8b105e` | Document current-account permission research and evidence gaps | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6443d56` | Record current Harvest permission evidence and reference conflicts | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `e8dcb77` | Add typed permission catalog and built-in profile selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `b80f8ab` | Recheck administrator authority during user access changes | legacy-access-writers | Three exact Rust blobs and regenerated cache in #227; specification hunks retained for reconciliation |
| `b7e730c` | Define permission persistence and transactional access contracts | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `412035d` | Map permission operations and transaction constraints | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `324084c` | Clarify permission boundaries for jobs and authentication | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `5fb78a7` | Specify permission preservation when deleting templates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `f17fafa` | Clarify report-specific financial access | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `8455740` | Document resource-specific managed rate proposal | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `168d536` | Specify resource-scoped billable rate permissions | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `5bf841d` | Specify explicit cost rate permissions | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `ece40f5` | Clarify company lock scheduling and approval boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `7bcb3e2` | Require full record visibility for combined approvals | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `0fe8f56` | Document project manager assignment retention evidence | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `a1cc799` | Specify project manager retention with read access | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `9c07d60` | Clarify delegation evidence and approval history boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `ab9b1a4` | Define project-editor delegation and cross-feature permission contracts | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance; original New Project transition hunks applied over current expense requirements |
| `80e2e42` | Track person delegation and project lifecycle permission gaps | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `eb85bc1` | Specify migration preservation checks for assignment dependencies | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `b8a1ef5` | Record pending person-management policy decision | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `3cc9afe` | Restrict person-management assignment changes to administrators | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `524c29e` | Reject malformed stored permission selections | scope-domain | Code/tests in #219; specification hunks retained for reconciliation |
| `ec9408b` | Record pending person-assignment eligibility decision | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `d965718` | Require compatible grants for new person-management assignments | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `0c56dc6` | Document managed-person removal on Harvest role downgrade | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6474382` | Confirm person-assignment removal after permission loss | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `3a19a26` | Specify explicit project-access retention choice | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `91f7cc3` | Distinguish profile selection from unchanged permission saves | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `602b04f` | Separate displayed permission profiles from assignment provenance | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `1105b75` | Record remaining permission decision gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `cf5d635` | Define rejection of person-management self-assignments | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `804b1d8` | Separate permission increment readiness from activation gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `6e61593` | Validate person-management grant compatibility and self-links | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `ec35446` | Store versioned permission profiles without activating new policy | permission-storage | Storage/schema/name-validator code/tests in #222; specification hunks retained for reconciliation |
| `d50c979` | Add audited permission template commands | permission-profile-transactions | Command/test/receipt/cache hunks in #226 with later hardening; specification hunks retained for reconciliation |
| `f5e0dde` | Apply permission profiles with atomic scope changes | permission-profile-transactions | Model serialization in #222; strict template receipt comparison in #226; profile commands, migration 0044 and tests/cache in #234 with later hardening; specification hunks retained |
| `9a7e05d` | Add audited project manager delegation | project-manager-delegation | Final command and all production-command tests in #243 (`3404c85`), including later hardening/composition; suite/cache/format, final WASM lint and full local Nix passed; specification hunks preserved separately |
| `fc85231` | Add administrator-only permission audit lookup | permission-audit | Complete reader/model/tests in #245 (`533922a`); suite, complete cache provenance and offline lints passed; full Nix/browser pending; specification hunks retained |
| `c3d17cb` | Prevent deadlocks during legacy import report conversion | import-transaction-lifecycle | Conversion source/tests/SQLx in #223; specification hunks retained for reconciliation |
| `3ae8e08` | Coordinate project access changes before locking resources | legacy-access-writers | Rust/test/cache changes in #228; organization SHARE query also reused in #220; specification hunks retained |
| `907bc88` | Recheck authority when saving organization branding | branding-authority | Source/test/cache hunks in #225; specification hunks retained for reconciliation |
| `d7a5a21` | Revalidate administrator authority for Harvest connection changes | import-authority | Eight Rust/test files and regenerated cache in #235 on #228; suite/SQLx/offline lints/format/full local Nix passed; specification hunks retained for reconciliation |
| `4fac6af` | Revalidate import job command and status authority | import-authority | Exact executor-based `jobs::cancel` owned by #231; remaining command/status Rust changes with the subsequent shared guard in #236; specification hunks retained for reconciliation |
| `b4672a4` | Revalidate authority during import error downloads | import-authority | All eight combined command/download Rust/test files byte-identical in #236, plus eight regenerated SQLx additions matching original; suite/cache/offline lints/format/full local Nix passed; specification hunks retained |
| `e5fcc5a` | Prepare durable CSV batches before opening transactions | import-transaction-lifecycle | Two exact Rust blobs and regenerated cache in #231; local verification passed, CI pending; specification hunks retained |
| `482b7c5` | Retain the original requester of import jobs | import-requester-provenance | Seven original source/schema/test paths and exact regenerated cache patch in #237 on combined #236/#222 prerequisites; 1,218 tests, schema order, cache, offline lints and format passed; full Nix passed on unchanged diagnostic rerun; initial inherited-menu failure retained; specification hunks retained |
| `e949e4c` | Drain interrupted import transactions before releasing reservations | import-transaction-lifecycle | Production/test/cache hunks in #224; specification hunks retained for reconciliation |
| `c0cfb8f` | Scope rate permissions to their owning resource | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `5d51b0e` | Expose the current person's permission snapshot | own-permissions | Reader, DTO, complete DB/HTTP tests and registrations in #244 (`6c4e4d1`) on #234; suite, cache, offline lints, isolated browser and full local Nix passed; specification hunks retained |
| `22ffdab` | Recheck manager access for financial snapshots | manager-snapshot-consumers | Shared helper/queries in #220; consumer/tests/cache in #232, local suite, both target lints and full Nix passed; specification hunks retained |
| `15c82ef` | Recheck manager access in invoice editor snapshots | manager-snapshot-consumers | Consumer/tests/cache in #232; fixture adaptations documented, local suite, both target lints and full Nix passed; specification hunks retained |
| `dc822a0` | Recheck manager authority before delivering exports | export-authority | Materialized XLSX/PDF source/tests and regenerated original cache in #247 (`d9717e7`); shared helper already in #220; 1,229 tests, complete cache, offline lints/format passed; full Nix pending; mixed specification hunks retained |
| `108594e` | Revalidate project scope before delivering exports | export-authority | Historical project reader and complete original DB/HTTP tests in #247 (`d9717e7`); suite/cache/lints passed, full Nix pending; later canonical changes and mixed specification hunks separately retained |
| `dcf4ac8` | Recheck current permissions during CSV downloads | export-authority | Legacy CSV cursor/batch-release source, migration 0046, original tests and regenerated cache in #249 (`71232dc`) on #247; 1,245 tests, cache provenance, format and offline lints passed; full Nix `72947` passed; later canonical stream changes and mixed specification hunks retained separately |
| `0793ce7` | Revalidate budget email authority before delivery | budget-email-authority | Four source/test files byte-identical in #238 (`7a7cede`) on master `1b8fa4f`; 1,133 tests, SQLx, format and offline server/WASM lints passed; full local Nix passed; specification hunks retained |
| `8aac739` | Add read-only permission migration diagnostics | permission-preflight | Reader and all seven tests byte-preserved in #246 (`ff482c8`) on #237/#226 review base; 1,244 tests, full cache/provenance, offline lints and unchanged-head full Nix rerun passed; initial inherited browser timeout retained; specification hunks in #248 |
| `8c15bfe` | Show own permissions in Settings | own-permissions | Complete original component, shared descriptions, SSR and resource tests in #244 (`6c4e4d1`); suite/cache, isolated browser and full local Nix passed; no full T018 claim; specification hunks retained |
| `300d1e9` | Expose administrator permission history | permission-audit | Complete historical DTO/HTTP/fencing tests in #245 (`533922a`); suite, complete cache provenance and offline lints passed; full Nix/browser pending; specification hunks retained |
| `03e90b1` | Connect permission editor previews and commands | permission-editor | Template commands/helpers in #226 and person commands/calculation in #234; final editor DTOs, session API, reader and original DB/HTTP tests in #250, with #245 strict audit dependency; source suite/cache passed, final gates pending; specification in #248 |
| `7f7fd1c` | Record permission editor delivery and UI follow-up | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `98b1692` | Add reviewed person permission editing | permission-editor | DTO runtime in #250; web-expectation removal, final editor state/export wiring and complete UI tests now preserved in #260 `98857cf`, executable gates pending; specification in #248 |
| `9e6d8bd` | Record person editor delivery and template follow-up | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `0f97cb2` | Add custom permission profile controls | permission-editor | Template DTO runtime in #250 using #226 commands; template controls and web-expectation removal now preserved in #260 `98857cf`, executable gates pending; specification in #248 |
| `8db19ba` | Show affected names in permission reviews | permission-editor | Profile command's relationship-type reuse in #234; final loss-label DTO/reader/DB/HTTP portions in #250; final page/draft/rendering tests now byte-preserved in #260; specification in #248 |
| `e7d8a36` | Protect permission drafts during navigation and dismissal | permission-editor | Final editor/recovery files in #260; navigation guard preserves current Clients. Cross-#259 union retained and verified in native composition f90f60f7, including all13 navigation assertions; this does not replace current-head remote checks. Specification in #248 |
| `6bba224` | Bind permission saves to the original requester | permission-editor | Shared requester DTO already in prerequisites; final session-save binding and HTTP assertions in #250; final editor/recovery UI portions now byte-preserved in #260; specification in #248 |
| `1ecfa21` | Recover interrupted permission saves across reloads | permission-editor | Final recovery/storage/template/UI fixtures byte-preserved in #260; runner retains current suites; specification in #248; full gate pending |
| `c88ca6d` | Exercise permission recovery in a real browser | permission-editor | Final real-browser recovery fixture and runner wiring in #260; specification in #248; full gate pending |
| `202ee96` | Protect project delegation against concurrent deactivation | project-manager-delegation | Complete final command/activity tests in #243 (`3404c85`); suite/cache/format, final WASM lint and full local Nix passed; specification hunks preserved separately |
| `3f45b7c` | Validate combined approval record coverage | scope-domain | Code/tests in #221; specification hunks retained for reconciliation |
| `eb56af3` | Define scoped approval transaction and coverage gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `8d49421` | Add authorized permission editor subject discovery | permission-editor | Complete final subject DTO/API/reader, 305-line DB tests and HTTP assertions in #250; exact source/cache provenance and workspace suite passed; final gates pending; specification in #248 |
| `b735b3a` | Add safe person switching to permission editor | permission-editor | Final subject DTO runtime content in #250; person-switching consumer and historical web-expectation removal now byte-preserved in #260; specification in #248 |
| `7f14e4b` | Limit user directory responses to consumed fields | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `1eb13ec` | Add scoped people directory reads | people-directory | Reader, DTO, session endpoint, seven DB tests and canonical HTTP assertions extracted in draft #253; exact-head tests/Clippy/SQLx and full local Nix gate passed on 3a37538; combined integration remains pending. Specification owned by #248; legacy HTTP assertions remain with #240 |
| `981d0e3` | Resolve approval names without directory access | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `6b5dbae` | Authorize identity-only project team choices | project-team-choices | DTO, reader, endpoint, nine DB tests, HTTP tests and ten SQLx descriptors extracted unchanged in draft #254 on #253; full native Nix `81722` passed on `f498c3f`; combined browser/deployment pending; specification owned by #248 |
| `4c00660` | Document project form permission integration boundaries | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `3bb62ac` | Limit session identity responses to display fields | identity-projections | Source/tests together in #240 at `1ce993f`; 1,122 tests, regenerated cache, format and offline lints passed; full Nix running; canonical tests and mixed specification hunks separately preserved |
| `4ac30fa` | Add scoped time-entry reads without financial metadata | time-readers | Original DTO, reader, endpoint, eight DB tests and HTTP assertions in draft #255 `d93e1af` on `0117991`; 23 original SQLx descriptors preserved, module registrations adapted only; formatting/provenance and full native Nix `40092` passed; wider composition pending; specification owned by #248 |
| `c80233b` | Keep invoice identities out of time-entry responses | time-entry-payload | Exact final model and independently registered original legacy HTTP assertions in #242 (`43337fc`); 1,122 tests, complete SQLx, format, offline lints and full Nix passed; canonical fixture remainder and specification hunks separately preserved |
| `5ec183a` | Fence time-entry writes against account deactivation | time-writer-activity | Source/tests and regenerated cache in #241 (`7820f8d`), original configuration SQL inlined without canonical module; 1,142 tests, both offline lints and full Nix passed; specification hunks retained |
| `228e151` | Clarify timesheet context and locked calendar behavior | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `4294aa3` | Isolate permission browser fixtures and retain test assets | browser-fixture-tooling | Original browser.nix asset-path hunk in #259 `455c155` and equivalent independent #260; single-admin DEV_LOGIN and viewport synchronization in #260's byte-identical permission recovery fixture; New Project manager fixture and single-admin assertion in #269's byte-identical browser fixture. All three original paths accounted for |
| `3308926` | Keep permission profile name uniqueness independent of database locale | permission-storage | Migration/storage regressions in #222; command lookup changes remain with template commands |
| `8af562e` | Record passing permission regression gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `9b53182` | Verify profile capacity and confirm timesheet discovery | permission-editor, specification-history, time-readers | Original 50-contender HTTP capacity assertions retained in #250's exact final test file and passed in its workspace suite; specification/Timesheet discovery decision in #248, not a claim to deliver later Timesheet implementation |
| `60f60f9` | Add scoped Timesheet person discovery | time-readers | DTO, reader, eight DB tests, endpoint and HTTP additions extracted in draft #256 `1552fdb` on combined #255/#253 review base `40102ae`; five original SQLx descriptors; formatting/provenance, tests/Clippy/SQLx `61768` and full native Nix `37414` passed; wider composition pending. Legacy HTTP block remains owned by #242; specification owned by #248 |
| `5faed76` | Bind Timesheet page reads to requester and subject | time-readers | DTO, reader, six DB tests, endpoint and HTTP additions extracted in draft #257 `8e09e60` on #256 `1552fdb`; all queries reuse existing descriptors; formatting/provenance, tests/Clippy/SQLx `63162` and full native Nix `72055` passed; wider composition pending; specification owned by #248 |
| `e1ddd9a` | Connect Timesheet to complete scoped page reads | timesheet-consumer-commands | Original complete-page loading, drafts and refresh tests in #259 `0dfea8b`; final page/helpers copied exactly. Consumer-only DTO lint removals included. Native/full and combined verification pending; specification owned by #248 |
| `48a6533` | Define person-bound Timesheet command contracts | timesheet-consumer-commands | 67-line contracts extracted with implemented endpoints in draft #258 `d2b45ca`; no standalone stub delivery. Formatting/provenance passed, executable gates running |
| `02c4245` | Authorize person-bound Timesheet commands atomically | timesheet-consumer-commands | Whole command module,13 DB tests, HTTP assertions, implemented endpoints and39 original SQLx descriptors in draft #258 `d2b45ca`; source/format passed, `64524` running. Two DTO web-expectation removals stay with connected UI; approval-covered editing remains incomplete |
| `a0632a8` | Bind Timesheet navigation and actions to the selected person | timesheet-consumer-commands | Original navigation/person/tracking/command UI in #259 `0dfea8b`; final helpers/page exact, route and shared guards adapted to preserve current Clients/audit work. Navigation harness retains every assertion with client coverage substituted for the still-pending permission-editor branch |
| `b8b1c60` | Bind weekly submission to the active Timesheet context | timesheet-consumer-commands | Original weekly-submission context contract, page caller,124-line DB race tests,97-line HTTP fixture and five cache descriptors in #259 `0dfea8b`; obsolete descriptor replaced recoverably. Legacy-own submission boundary unchanged; specification owned by #248 |
| `8722320` | Add browsable permission change history | permission-audit | Own-reader authentication-error sanitization in #244 (`6c4e4d1`); audit reader/UI/navigation, Settings link, profile labels and shell tests in #245 (original extraction `c96d787`); editor consumer and the later real-writer/history browser assertions belong to #260. Specification owned by #248 |
| `2078a13` | Format permission history verification notes | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `2f5357f` | Verify permission history and scoped Timesheet browser flows | browser-fixture-tooling, permission-audit, timesheet-consumer-commands, permission-editor | Timesheet readiness and New Project Timesheet-picker browser hunks in #259 `0dfea8b`, retaining current master Clients/project assertions. Complete permission-editor/history fixture owned by #260 and byte-identical to original final fixture, including all96 lines added here; specification owned by #248 |
| `68bbaae` | Fix Timesheet modal focus and long-label layout | timesheet-consumer-commands | Original modal focus and long-label picker fixes plus modal/error browser suites in #259 `0dfea8b`. Runner preserves current suites and adds only applicable Timesheet/navigation suites; specification owned by #248 |
| `e29f4d8` | Reload Timesheet state when switching people | timesheet-consumer-commands | Final original keyed person-switch remount and browser-history assertions in #259 `0dfea8b`; full browser gate pending; specification owned by #248 |
| `5f7895c` | Preserve selected dates and drag offsets in Timesheet | timesheet-consumer-commands | Final selected-date/calendar code and assertions in #259; initially omitted original two-line pointer-target CSS restored in `4ce0919` after unchanged test exposed the dependency. Full gate `75088` pending; specification in #248 |
| `84d5352` | Expose authenticated project manager delegation | project-manager-delegation | DTOs, session wrappers, reader and tests in #243 (`3404c85`); one web-only DTO lint expectation is the recorded extraction adaptation; HTTP audit-visibility block and audit-fixture adaptation remain owned by permission-audit delivery; specification hunks retained |
| `c4e83c8` | Record project delegation verification and next integration gate | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `a25e544` | Serialize invoice writes before user revocation | legacy-access-writers | Five Rust/test changes and eight SQLx additions in #233 on integrated #220/#227/#228/#232 prerequisites; suite/cache/offline lints/full Nix passed; specification hunks retained |
| `774f60a` | Record invoice revocation verification and next integration gates | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `1879b8a` | Preserve requester identity when reloading permission editors | permission-editor | Final requester-bound editor and complete browser/UI tests byte-preserved in #260; specification in #248; full gate pending |
| `dab6885` | Record editor reload verification and remaining directory integration | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `ee16165` | Connect scoped People directory and requester-bound editing | people-directory | People consumer, original DTO web-expectation removals/import spelling, historical admin/shell/sidebar and UI fixture preserved in #260. Later task-catalog wiring stays separate; gates pending |
| `1b41033` | Record People integration verification and remaining report scope | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `75f13a1` | Add scoped detailed time report reads | time-report-readers | Reader/DTO/endpoint and detailed DB/HTTP assertions in #261 bf452dd; final reader includes totals/filter refinements, grouping separate; specification in #248; full gate pending |
| `cbc78a8` | Apply scoped permissions to time report spreadsheets | time-report-exports | XLSX reader/release scope,11 unchanged DB tests, HTTP assertions and original snapshot-test adaptation in #2638cc11c3; cache reconciled with final query predicates. Specification preserved in #248; full gate pending |
| `09bd15f` | Apply scoped permissions to streamed time exports | time-report-exports | Native stored-row decoder in #222; scoped streaming source/delivery, shared release helpers and10 DB tests/HTTP assertions in #26455d362b; specification in #248 |
| `93aaa68` | Support multi-selection filters in time downloads | time-report-exports | Shared URL parser, five unit tests and238-line two-format HTTP fixture in #26455d362b; specification in #248 |
| `a23804f` | Include full-period totals in scoped time reports | time-report-readers | One-statement totals, detailed DB/HTTP pagination assertions and exact cache in #261 bf452dd; specification in #248; full gate pending |
| `7266abb` | Connect scoped time reports with bound downloads | time-report-consumer | Backend preflight/DTOs, CSV/XLSX mode binding and parser/HTTP tests in #265f436a29; UI, browser/component fixtures and consumer lint removals in #268810ce57; specification in #248 |
| `41ff137` | Add scoped time report grouping with exact totals | time-report-readers | Grouped reader/contracts,13 final DB tests and original171-line HTTP fixture in #26266dbf0b, full gate79456 passed; export_filters helper additions in #2660eec1a4; UI retained separately; specification in #248 |
| `e2e66fb` | Connect scoped time report groups and detail navigation | time-report-consumer | Grouped consumer and original browser/component assertions in #268810ce57; specification in #248 |
| `ca170c0` | Export scoped time groups to Excel with release authorization | time-report-exports | Grouped XLSX reader/renderer/route and seven DB tests in #2660eec1a4; UI/browser/component assertions owned by #268; specification in #248 |
| `2b59b58` | Stream grouped time reports with scoped authorization | time-report-exports | Grouped CSV cursor/delivery/route, nine DB tests and shared group-lifetime authorization in #2660eec1a4; UI/browser assertions owned by #268; specification in #248 |
| `ecac66b` | Add scoped individual time reports and nested breakdowns | time-report-consumer | Three original scoped modules, individual/nested transitions and original browser/component assertions in #268810ce57; specification in #248 |
| `de8f9ad` | Filter time reports to active projects | time-report-consumer | DTO/readers and reader tests in #261/#262; export SQL predicates in #263/#264/#266; strict URL transport and original cross-format snapshot/authority fixtures in #267e029a89. UI/browser/component hunks owned by #268; specification in #248 |
| `2497dbe` | Enforce scoped permissions in the project editor | project-editor-permissions | Pure RateEdit code/tests in #221; composable delegation and transaction tests in #243; picker reader in #254. Canonical editor DTOs, field/association/save logic, UI, DB/HTTP/browser tests and consumer lint adaptations extracted in #269. Full acceptance remains pending; later task lifecycle changes are not included. Specifications owned by #248 |
| `2631186` | Enforce scoped project reads across pages and exports | project-read-permissions | Ordinary readers, budgets, minimal labels and bound overview/detail UI in #270; CSV/XLSX release boundaries in #271; Harvest-compatible project list/count/direct-ID readers in #272. Shared export helpers compose over earlier export PRs. Tests and original descriptors retained; fixture adaptations and failing gates recorded below. Final shared-file hunk audit and complete acceptance remain pending; specifications owned by #248 |
| `1b81680` | Record project permission delivery acceptance | specification-history | Feature-015 final document state in #248 (`c77abf9`); complete historical revisions preserved in original refs; no runtime or full-feature acceptance |
| `f6e8bf1` | Enforce task catalog and tracking read permissions | task-permissions-lifecycle | Task catalog/tracking and Harvest-compatible readers, DB/HTTP/browser fixtures, original descriptors and sidebar interaction correction extracted in #273. Later creation/rate commands belong to #275, lifecycle to #276, linking to #277, atomic creation to #278, catalog UI to #279 and project task activity UI to #280. Current-head acceptance remains separate. Specifications owned by #248 |
| `8dd61d4` | Enforce current task creation and project scope permissions | task-permissions-lifecycle | Creation commands and original DB/HTTP tests extracted into draft #275f8c1477; full native71588 passed, ARM/wider review pending. Later atomic creation from dcadcee is extracted in #278, with its catalog consumer in #279 |
| `facfb49` | Protect task rate edits with explicit intent and current permissions | task-permissions-lifecycle | Explicit rate transport, commands, original mutation/DB/HTTP tests and contract extracted into draft #275f8c1477; full native71588 passed, ARM/wider review pending |
| `ac4c90c` | Authorize task activity changes and guard running timers | task-permissions-lifecycle | Commands, original tests/descriptors and contract extracted with0591407 in draft #276924dcbe. Fresh gates pending |
| `0591407` | Preserve project task archival across restores and imports | task-permissions-lifecycle | Migration0048, import/checkpoint/lock-order adaptations, editor transport, tracking admission, original tests/descriptors and contract in draft #276924dcbe. Cache correction/fresh gates pending; later UI remains separate |
| `979a594` | Enforce scoped project task linking and rate currency | task-permissions-lifecycle | Commands, five DB regressions, HTTP matrix, original descriptors and contract extracted in draft #277d1ab522. Live-schema SQLx passed; tests/full gates pending |
| `5561f14` | Add permission-aware task catalog management | task-consumers | Catalog reader, DTOs, navigation/UI, tests/cache and contract in #27946d3b36; historical progress/quickstart owned by #248/original refs |
| `dcadcee` | Add atomic task creation to the task catalog | task-consumers | Backend, legacy caller, DB/HTTP tests and descriptors in #2783d76f99; catalog UI/browser/component tests and contract appendix in #27946d3b36. Historical progress/quickstart owned by #248/original refs |
| `8c1bf9b` | Add task archive and restore controls to project editing | task-consumers | Two UI files, original Chromium regression increment and contract appendix in draft #28024c5d0e; full native14092 running. Historical progress/quickstart retained with #248/original refs |
| `db3935d` | Filter time reports and downloads by billability | time-report-consumer | DTO/readers and reader tests in #261/#262; export SQL predicates in #263/#264/#266; grouped HTTP route registrations in #266; strict URL transport and original cross-format DB/HTTP fixtures in #267e029a89. UI/browser/component hunks owned by #268; specification in #248 |

Additional original #217 commit `dd141c5`: replaced by #220 for delivery, with
its original branch left untouched and open. All production/test changes are
carried with the documented #216 adaptation. Its existing feature record is
preserved with historical evidence explicitly separated from new-head results.
Seven of its eight added SQLx descriptors are emitted by the new preparation;
`query-7d5c8693570ee23f36fdb48ff0c4412ba250383ee2fa399d94fefe9d16767381.json`
already exists in `02f7b58`. Do not merge both #217 and #220 as separate fixes.

## Iteration log and next action

The ledger is published as draft [#218](https://github.com/numtide/horae/pull/218)
at `c0ce2bd`. Subsequent delivery results will update that same PR; no separate
tracking documents are needed.

### 2026-10-06 — Inventory, gate and recoverability

Read-only inventory ran while #216 was queued. The existing watch process
observed queue CI success without rerunning it or requesting a merge.
The pre-existing queue request merged #216; this separation work performed
no merge. Then backups and recovery checks passed, and two clean isolated
worktrees were created from the verified merge.

The ledger contains all 145 original commits and all 18 unpublished paths.
Targeted `nix fmt` passed. The dev shell does not expose `mdformat` directly;
the repository formatter was used instead. No Rust extraction tests or
new-head Nix gates have run yet.

### 2026-10-06 — Legacy-reader extraction started

Extracted the shared manager snapshot and all seven reader regressions plus the
registered-session test from #217. The three production readers now use that
snapshot. The #216 invoice query, optional client filter and stable ordering
remain unchanged. The HTTP harness receives only the exact endpoint matcher
and the reader cases; no unrelated canonical consumers are copied.

Necessary adaptation: inline the exact organization SHARE query from the
existing helper into snapshot authorization. The two test fixtures retain the
original NO KEY UPDATE query directly. This avoids importing unused writer
modes or modifying master writers merely to support the extracted reader.
No schema, new dependency, UI, policy activation or ordinary writer changes.

Source review confirms current user revocations acquire the organization lock
before user rows; the extracted readers take organization then actor SHARE and
commit before returning. Invoice headers/lines use the same repeatable-read
transaction. Full adversarial acceptance and runtime results remain pending.

Focused validation is running in session `63131` through the Nix shell, using
`.scratch/verify-readers.sh` in the extraction worktree. Each invocation starts
a fresh private PostgreSQL cluster and applies only the base migrations there;
no existing database is used. It reuses ignored Cargo artifacts, not source
files, from the existing target directory. No test has been declared passed
while dependency compilation is still running.

### 2026-10-06 — Focused reader regressions passed

Session `63131` completed successfully: all seven snapshot reader tests passed
(2.96 seconds after the initial 10m57s build). They exercise all three readers,
both revocation orders, direct and organization-gated writers, tenant/filter/
not-found behavior, coherent invoice headers/lines, cancellation and inherited
connection settings. Other test binaries were filtered, not verified by this run.

`nix fmt` completed successfully, changing only the new snapshot module's
formatting. The post-format full workspace server/core suite is running in
session `71462`, through the same disposable-database wrapper. This includes
the registered HTTP harness and #216's client regressions; no result is claimed
until it finishes. No original source worktree was changed.

Next: collect the full suite, regenerate SQLx and run offline/lint/Nix gates.
Review and publish the scoped PR only with honest readiness/evidence, then
continue the remaining behavior groups. The next foundation candidate is the
pure scope/catalog code: retain master’s new `client` module when adding
`permissions`, and leave rates/approval/person-management consumers with their
actual dependencies. Do not copy the old core module root wholesale.

### 2026-10-06 — Pure scope/catalog extraction started

Created `refactor/permission-domain-foundation` independently from `02f7b58`.
Reused the existing source and tests without changing their semantics. Reviewed
the existing `record-scope.md` and `grant-catalog.md` contracts: coverage denies
inactive/foreign/misattributed facts, assignments alone grant nothing, saved
selections reject missing prerequisites rather than adding authority, and an
Administrator selection does not prove Administrator identity. There are no
runtime callers in this delivery and no legacy role conversion or activation.

Core tests run in session `34224` with a separate temporary Cargo target, so
the reader suite's binaries and artifact lock are not disturbed. Formatting
check is session `29855`. Results and the resulting PR are still pending.

### 2026-10-06 — Full reader suite and foundation delivery

Reader session `71462` finished with exit zero: 826 app-unit tests, 180 tests
across the nine integration binaries and 121 core tests passed (1,127 total).
The 11 existing ignored scale/stress tests remain unchanged, not counted as passed.
This includes the authenticated reader cases, all seven snapshot regressions
and all 35 Clients tests from #216. No Rust source changed after this run.
SQLx preparation now runs in session `84393` against another private database,
with `--workspace -- --features server --all-targets`. All test entry points
were touched before preparation to avoid losing cached test-query metadata.

Foundation sessions `34224`, `29855` and `11685` finished successfully:
158 core tests (37 extracted plus 121 existing), `nix fmt -- --ci`, and core
Clippy with all targets, warnings denied and performance lints. All five new
source blobs match `524c29e` exactly; the sixth file adds only the module export.
Unsigned commit `ec7ddbd` is published in draft #219. It is independent of the
reader extraction and does not change app behavior. Full Flake Check remains
pending; local core checks alone do not make it ready to merge.
Its head was confirmed as `ec7ddbd1f1271db523abc35d75a82d8b6fc3bb29`;
CI run [37474017234](https://github.com/numtide/horae/actions/runs/37474017234)
started Flake Check and Format. No outcome is assumed from their running state.

Next: collect SQLx preparation, verify offline all-target compilation and lint,
then publish the reader extraction. Collect #219's own CI before accepting it.
Continue the remaining source groups and specification-hunk reconciliation;
the original PRs remain open and unchanged.

### 2026-10-06 — Independent reader PR published

SQLx session `84393` passed, preserving all 1,003 existing descriptors byte for
byte and adding 18. Offline server Clippy with all targets passed in `85490`;
WASM Clippy passed in `53119`, both with warnings denied, performance lints and
`DATABASE_URL` unset. Final `nix fmt -- --ci` passed in `59584`.
No Rust source changed after the full test run; only generated SQLx metadata
and the carried feature verification record were added afterward.

Unsigned commit `bc0c7a0bf2822c1e61568a95b53df651fe40e7c4` is published as draft
#220. Its 27 paths comprise eight Rust source/test paths, 18 SQLx descriptors
and the existing feature record. Adversarial review found no high/critical
issue in the extracted source; that is not a replacement for the pending
required Flake Check. #219 and #220 are independent deliveries from the same
#216 base, so neither requires the other to merge.

Next: collect the exact-head CI outcomes for #219/#220 without frequent polling.
Continue with domain-dependent rules and storage extraction, keeping storage
policy at zero and carrying the complete relevant tests. Reconcile the remaining
specification hunks and every source group; the overall separation is unfinished.

### 2026-10-06 — Dependent domain rules published; storage isolated

#221 contains the six unchanged source/test files for person-management
prerequisites, financial field gates (including explicit rate intent) and combined
approval record coverage, plus their module exports. All source blobs match
`db3935d`. Its 187 core tests passed, including the 158 tests of #219 and 29
additional tests; core all-target Clippy and formatting passed. These predicates
do not implement assignment writes, rate mutations or approval lifecycle.
Their documented trusted-input, complete-selection and independent lock boundaries
remain unchanged; source review found no high/critical issue within this scope.

The repository Actions workflow filters pull requests to base `master`.
Consequently #221 does not inherit #219's check result and cannot claim its own
Actions run. Full `nix flake check -L --max-jobs 1 --cores 2` is running locally
in session `98201` on committed head `539316c`. Required CI must run again after
eventual retargeting; no workflow filter or required check was weakened.
At the latest observation #219/#220 Format and nix-eval passed, while their
full Flake Check/build jobs remained in progress.

Storage is a separate sibling branch on #219, not dependent on #221. It carries
unchanged migrations 0042/0047, the final strict storage loader/models and storage
regressions, and the original profile-name validator. Command modules, transaction
configuration helpers and runtime readers are deliberately excluded. The existing
non-test dead-code expectation documents that this internal storage is not yet
activated; no test is disabled or weakened. Serializer additions originate in
`f5e0dde`; the native stored-row decoder originates in `09bd15f`. Those source
commits' commands/export consumers remain separate pending work.

The migration identities and checksums are retained. A private database first
applied this subset (42/47), then the preserved original migration directory:
43–46/48 applied successfully and all seven versions were verified present.
This verifies deferred migration compatibility without renumbering migrations
or modifying a real database. Full storage server/core tests are now running in
session `96597`; formatting passed unchanged. SQLx/offline/lint/Nix acceptance
for storage remains pending, and no storage PR has been published yet.

Next: collect storage tests, regenerate its SQLx cache and verify offline/lint;
collect the live local #221 Flake Check without restarting it. Then publish the
storage extraction and continue command/read-consumer groups and spec reconciliation.

### 2026-10-06 — Storage published; independent conversion repair started

Storage session `96597` finished successfully: 830 app tests, 180 integration
tests and 160 core tests passed (1,170 total), with the 11 pre-existing manual
scale/stress tests still ignored. This includes all 11 original storage tests
and both original name-validation tests. No Rust source changed after that run.
SQLx preparation passed in `31386`: all 1,003 base descriptors are unchanged;
the 19 additional descriptors match the original `db3935d` blobs exactly.
Offline all-target server Clippy (`2494`), WASM Clippy (`92443`) and final
formatting (`39928`) passed, with warnings denied and no format changes.

Unsigned commit `e9695fda224d1f8bc22e8e7fb7e8fc0a43fa4625` is published in
draft #222. Its 28 paths comprise nine Rust/schema paths and 19 query descriptors.
Storage review checked composite tenant FKs, exact restoration without inference,
restricted template deletion, non-activation, Unicode collision rollback and
the absence of endpoints/callers. No high/critical finding within that boundary;
future command authorization, profile capacity and cutover remain out of scope.
The two migrations, models, storage tests and final catalog files match the
preserved final source. Full local Flake Check is running on this committed head
in session `50374`; master-only Actions filtering still requires later CI.

The existing #221 local Flake Check (`98201`) remains live: release server/WASM
build completed and browser checks are progressing. This is not a terminal pass.
The latest GitHub observations still show #219/#220 Flake Check and nix-build
running, with Format and nix-eval successful. No check was restarted or bypassed.

Created an independent `fix/import-report-lock-order` worktree from `02f7b58`.
Its two source/test files match `c3d17cb` exactly: discovery without the job lock,
organization SHARE before job UPDATE, READ COMMITTED revalidation and rediscovery
after another worker/converter changes the candidate. All five original races
are retained. Existing size-one pool, archived-error and state-preservation tests
remain intact. The later `482b7c5` requester-provenance assertions stay with that
schema/feature, not this extraction. No original worktree was modified.
Full workspace server/core tests are running on a private database in `58270`.

Next: collect conversion tests, regenerate its SQLx cache, complete adversarial
and offline/lint checks, and publish its independent draft PR. Collect the live
#221/#222 Nix outcomes without restarting either process. Continue the remaining
behavior groups and specification-hunk mapping; the overall separation is not
complete and no merges or original-PR closures have been performed.

### 2026-10-06 — Conversion repair published; session cleanup isolated

Session `58270` passed the full conversion workspace suite: 824 app tests,
180 integration tests and 121 core tests (1,125 total), with 11 pre-existing
manual scale/stress tests ignored. All five original concurrency regressions
passed, along with the existing legacy conversion, size-one pool and Clients
regressions. Formatting (`33716`) passed unchanged. SQLx preparation (`62415`)
preserved 1,002 base descriptors, removed the superseded locking-discovery
descriptor and added 14 including its replacement (1,016 total). All 14 match
the original `c3d17cb` source. Offline all-target server Clippy (`33078`) passed;
WASM Clippy (`79587`) also passed. No Rust source changed after the full suite.

Unsigned commit `c8f95ac5586dfed5549d4e394516f12b02339741` is published as draft
#223 on master. Adversarial review traced the one-connection conversion and its
archive/checkpoint, claim, cleanup and startup callers: exact tenant/job recheck,
fresh post-wait payload, compatible organization SHARE versus worker FK KEY SHARE,
no duplicate archive or invalidation of an already-converted live lease, and
atomic failure rollback. No high/critical issue was found in this extraction.
This accounts for T065–T067's code, not full T039/T040/T042 or policy activation.
Required exact-head CI remains pending.

The next independent branch carries exactly the `e949e4c` source/test hunks;
stable patch ID `3168407d233c130834dd0fa7328552224993c1f9` matches the original.
It drains abandoned SQLx responses, tolerates only disposal-time `3B001`, sends
full ROLLBACK before advisory unlock and closes the reserved connection. The
original tests cover one/two nested savepoints, an untracked server transaction,
retained committed values, immediate single-connection retries and backend failure.
Source review checked the pinned SQLx 0.8.6 flush/transaction implementation and
both API/CSV worker-join paths against the existing T089–T091 contract. Format
passed in `66687`; runtime/SQLx/lint acceptance is still pending.
The full workspace server/core suite passed in session `31728` through
`.scratch/verify-cleanup.sh` on a fresh private PostgreSQL cluster: 822 app,
180 integration and 121 core tests (1,123 total), with 11 pre-existing ignored.
All three cleanup regressions and existing API/CSV cancellation regressions
passed. No source changed afterward; SQLx preparation is the next gate.

Also isolated the independent branding-write repair on `02f7b58`. Both files
match `907bc88` exactly, carrying all five new authority/rollback regressions and
all prior field/no-op tests. The only production caller passes its authenticated
actor ID; the transaction retains organization UPDATE before actor SHARE, checks
current active same-tenant Manager/Admin authority even for no-ops, and commits
before the unchanged conditional plugin event. Current role writers use the
compatible organization-first order. This neither changes the branding read
endpoint nor activates future CompanyWrite policy. Formatting (`81218`) passed;
runtime, SQLx, lint and PR publication remain pending.

The durable CSV preparation candidate `e5fcc5a` has a real test dependency on
`4fac6af`'s executor-based `jobs::cancel`: master accepts only a pool, while the
original cancellation race calls it inside its transaction barrier. Preserve
that production-call regression; do not replace it with a weaker fixture write
or copy unrelated import-authority commands merely to make the tests compile.
Resolve the small shared helper or its dependency when extracting that candidate.

Next: finish session-cleanup SQLx/offline/lint verification and its
scoped PR; then verify the branding extraction. #223's exact remote head was
confirmed and CI run [37479980256](https://github.com/numtide/horae/actions/runs/37479980256)
has Format/nix-eval passed, with Flake Check/nix-build in progress.
Keep collecting the live #221/#222 local
Flake Checks (`98201`/`50374`) without restarting them. All remaining source and
specification groups still require final mapping and verification.

### 2026-10-06 — Cleanup published; completed foundation gates

The preceding response supplied the requested goal text but made no repository
progress. Resumed the existing live processes rather than restarting them.
Session `22335` completed SQLx preparation successfully: all 1,003 existing
descriptors are unchanged and seven additions match `e949e4c` exactly. Offline
all-target server/workspace Clippy (`37045`) and WASM Clippy (`77529`) passed with
warnings denied and performance lints enabled. The source/test patch still has
the original stable patch ID. Unsigned commit
`be57f0e5ebf449f446aed69ad62a84e9ee5583b2` is published as draft #224, independent
on master. Required CI is pending; no original branch was changed.

Session `98201` completed `nix flake check -L --max-jobs 1 --cores 2` with
`all checks passed!` on clean #221 head `539316c`, including its #219 dependency.
This covers the compatible x86_64-linux checks, not the omitted incompatible
systems. The PR description now records the result; master-targeted Actions
must still run when the PR is retargeted. #222's existing full check (`50374`)
is still live and has passed its release build and progressed through browser
checks. It was not restarted.

Exact-head GitHub checks were queried once after new results became available:
#219 run [37474017234](https://github.com/numtide/horae/actions/runs/37474017234)
and #220 run [37475284999](https://github.com/numtide/horae/actions/runs/37475284999)
both passed Flake Check and Format. Their separate Nixbot builds were still in
progress; do not report every remote check green. #212/#217/#208 heads remain
the preserved originals.

Branding verification is running in session `45526` on a fresh private database,
reusing the existing ignored verification wrapper. No Rust changes were made
after the original-blob comparison. The next shared extraction is now isolated
on #222: receipt migration 0043, the complete template-command implementation and
all 19 original tests match `db3935d`. It also carries the exact three command DTOs
from `permission_editor.rs`, the existing bounded administration helpers and
module registration. Only those DTOs are server-gated at this stage; editor
screens, endpoints, profile application and policy activation are not included.
Formatting passed unchanged (`42874`). Tests and SQLx are not yet accepted.

Branding's complete workspace suite subsequently exited successfully (`45526`).
SQLx preparation (`53498`) passed with all 1,003 base descriptors unchanged and
six additions matching `907bc88` exactly. Offline server Clippy is now running in
`50256`; WASM lint and publication are still pending. The template extraction's
administration helpers were additionally compared byte-for-byte with `db3935d`.
Source review checked authorization-before-replay, strict canonical intent,
tenant/principal receipt separation, atomic count/name limits and deletion,
grant/identity preservation, revision overflow and rollback. No high/critical
source finding within the internal boundary; runtime acceptance is still pending.

Branding offline workspace/server Clippy (`50256`) and WASM Clippy (`26523`)
subsequently passed. Source files still match `907bc88` byte-for-byte. Unsigned
commit `f2d6bd45a9ef0ac141797d8745430bb0d8260f3e` is published as draft #225 on
master; required CI is pending. #224's exact remote head was confirmed; run
[37482514740](https://github.com/numtide/horae/actions/runs/37482514740) has Format
passed and Flake Check in progress (Nixbot evaluation passed, build in progress).

The template-command full suite is running in session `60494`, through the
existing private-database wrapper with quiet test output to keep complete result
summaries. It tests the combined #219/#222 foundation and commands, not a mocked
standalone command. No shared-target Cargo jobs are running concurrently.

Next: collect template-command tests, regenerate SQLx and verify offline/lint
before publishing the dependent PR. Continue collecting the live storage check
(`50374`, now running app tests after browser checks). Reconcile remaining shared
writers, consumers and all specification hunks before claiming this separation
complete.

### 2026-10-06 — Template command verification and storage integration gates

Previous iteration made progress: #224/#225 were published and the next internal
command extraction was isolated. Resumed its live suite (`60494`), which passed
849 app, 180 integration and 160 core tests (1,189 total), with 11 existing manual
scale/stress tests ignored. The 19 original command tests are included unchanged.
SQLx preparation (`86502`) passed with all 1,022 base descriptors unchanged and
41 additions matching `db3935d` (1,063 total).

The first offline Clippy run (`33332`) failed to resolve `horae_core::permissions`.
The source and committed core root both export it and were unchanged; this shared
target had previously compiled the master-only branding branch without that
module. Forcing recompilation by touching only `crates/core/src/lib.rs` made the
same all-target workspace/server command pass (`58343`), with no source, cache or
lint changes. Treat this as stale shared-target build evidence, not a product
fix or a reason to weaken checks. WASM lint is the next gate; the independent
Nix build will also verify the committed source in a clean build environment.

#222's live full Flake Check (`50374`) completed with `all checks passed!` on
`e9695fd`, including its #219 foundation, browser checks and NixOS end-to-end
checks. The PR body records that compatible-system result. Its master-targeted
Actions checks remain required after retargeting; Nixbot is still in progress.
One remote query confirmed #223–#225 have Format passed and Flake Check running;
no CI rerun or merge was requested.

Dependency inspection for subsequent deliveries: profile-application commands
consume the template commands and #221's `has_person_management_grant`; their
tests also exercise real template commands. Preserve that dependency rather
than replacing it with fixtures or duplicating the domain predicate. The next
independent legacy-writer extraction is `b80f8ab` (user creation/role/activity
reauthorization), followed by assignment/project writer coordination. Later
`a25e544` only broadens `change_user_role` visibility for invoice race tests; it
does not change this helper's organization query. Keep its invoice-specific
tests and wiring with that later delivery.

WASM Clippy passed (`21514`). Unsigned commit
`82d15f33d2b7376c0a0973367ad012fbac5e752d` is published as draft #226 against
#222, with 6 source/test/schema paths and 41 generated query descriptors. The
production command, receipt migration and complete test file still match the
original blobs. Full `nix flake check -L --max-jobs 1 --cores 2` is running in
session `46319` on this clean committed head. Required Actions checks will also
be needed after master retargeting; none were weakened.

Next: isolate the three Rust files of `b80f8ab` on master, review all original
user-authority and last-administrator regressions, then verify the extraction.
Their pre-change source matches `02f7b58` exactly, so no unrelated directory or
invoice code is needed. Collect the live #226 Nix check without restarting it.
Profile application still needs both the template stack and #221; resolve that
integration explicitly before extracting its consumers. The goal remains
incomplete: many source groups and specification hunks still lack final PRs.

### 2026-10-06 — User mutation authority extraction

The preceding response only supplied goal text and made no repository progress.
Revalidated the actual goal, #216's merge and the original preserved worktree;
the original 18 unpublished paths remain untouched. Resumed live #226 full Nix
check `46319` rather than restarting it; its client release build passed and
the overall check is still running.

Created `fix/user-mutation-authority` in `.worktrees/user-mutation-authority`
from `02f7b58`. Its three Rust files match `b80f8ab` exactly: user transaction
helpers, seven new authority regressions and the updated last-admin tests.
The two existing files at `b80f8ab^` are identical to the extraction base.
No directory DTOs, invoice-test visibility change, new policy or schema was
included. This extracts the implementation of original T030/T031; T032 requires
fresh evidence, not the original quickstart's historical results. Associated
specification hunks remain preserved for reconciliation.

Source review traced the authenticated actor IDs through all three wrappers,
organization-before-actor locking, same-organization target queries, last-admin
serialization, duplicate rollback and event dispatch only after commit. Tests
exercise completed/concurrent revocation for all three commands, foreign/missing
actors, authority lock lifetime and all existing last-admin cases using actual
PostgreSQL lock observations. Directory/rate exposure and later project-writer
coordination remain outside this boundary. No high/critical source finding in
this extraction; runtime gates are not yet accepted.

Formatting passed unchanged (`57627`). Complete workspace tests are running in
`59860` through the existing private temporary PostgreSQL wrapper. Forced local
source recompilation by touching core/app/test entrypoints before reusing the
shared target; no source change resulted. Next: collect this suite, regenerate
the complete SQLx cache, verify offline server/WASM lint and publish the scoped
PR. Continue collecting #226's existing Nix handle; do not restart it.

The full workspace suite subsequently passed (`59860`): 826 app, 180 integration
and 121 core tests, 1,127 total, with 11 existing manual checks ignored. Complete
SQLx preparation is running in `39314`. #226's release server/client build has
passed; its full check has moved on to Clippy and remains live in `46319`.

Next-boundary review: `757f43d`'s assignment repair adds an actor lock before
resource/FK work. Preserve its later `3ae8e08` organization/project lock-order
repair together with the affected project-editor, invoice and time-entry race
regressions; do not call the early assignment commit alone the finished writer
boundary. Most pre-change paths of `3ae8e08` match the current base; the project
creation helper differs because #216 introduced shared client validation, which
must be retained. Its user/editor race additionally depends on this user-write
extraction. These dependencies were inspected, not implemented or certified.

SQLx preparation passed (`39314`): all 1,003 base descriptors unchanged, four
additions byte-identical to `b80f8ab`. Offline all-target workspace/server Clippy
passed (`19350`) with warnings and performance lints denied. Unsigned commit
`142eda198f8c3aa94eef17285cfe6b19ca2f4b05` is published as draft #227 on master;
its three Rust files are unchanged from the tested extraction. WASM Clippy is
running in `59244`; required CI remains pending. #226's existing full check
continues through browser checks in `46319`.

WASM Clippy subsequently passed (`59244`). #227's remote head and draft/master
base were confirmed; [CI run 37486457726](https://github.com/numtide/horae/actions/runs/37486457726)
has Flake Check/Format running, Nixbot evaluation passed and build running. No
merge or rerun was requested. Next: extract the assignment/project coordination
boundary (`757f43d` plus `3ae8e08`) with its original race tests on #227, retaining
#216's client validation; verify that combined stack. Keep collecting #226's
live full check without restarting it. Remaining source/specification groups
still require mapping and verification; the goal is not complete.

### 2026-10-06 — Project writer coordination extraction

The preceding iteration made progress: #227 was published with its full local
suite, SQLx and both target lints passed. Reconfirmed #216's merge and resumed
#226's live `46319` check; browser checks continue, without a restart.

Created `fix/project-access-lock-order` in `.worktrees/project-access-lock-order`
on #227 `142eda1`. Applied the Rust changes of `757f43d` and `3ae8e08` together.
Fourteen of fifteen files match `3ae8e08` byte-for-byte; the sole difference is
the client validation already merged in #216, which remains intact. Original
assignment, task, editor, invoice-FK, entry-cascade and user-revocation tests are
preserved. No UI, schema, policy activation or real-data change is included.

The dependency on #227 is real: the editor/revocation race exercises its actual
actor-aware user creation and role-change helpers. This delivers original
T024–T026 assignment work and T068–T070 project-family coordination, not the
complete T042 lock hierarchy or canonical-permission enforcement. Specification
hunks remain preserved in the originals pending reconciliation.

Formatting passed unchanged (`6430`). Full workspace tests for this combined
stack are running in `75426` on a fresh private PostgreSQL cluster. Next: finish
source/trigger/caller review, collect tests, regenerate SQLx and verify offline
server/WASM lint before publication; retain #226's live full-check handle.

The suite passed (`75426`): 836 app, 180 integration and 121 core tests, 1,137
total, with 11 existing manual tests ignored. This includes all ten added
regressions and the dependent #227 tests. Source review traced every production
creation-actor/task-enablement caller, checked the retained editor isolation and
parent lock mode, the existing revision triggers/member cascades, tenant-safe
assignment recheck and post-commit events. No high/critical finding in this
bounded extraction; full canonical/historical-writer integration is not claimed.

Complete SQLx regeneration passed (`30003`): 28 additions match `3ae8e08`, four
superseded descriptors were removed and 1,003 base descriptors are unchanged
(1,031 total). The removed queries are the replaced unscoped assignment deletion,
two late currency SHARE reads and task-link resource query, not lost test cache.
Unsigned commit `0e1e6759cff0b80a046ced4694a78022f096ab3d` is published as draft
#228 against #227. Offline server lint runs in `17183`; full Nix check runs on
the clean committed stack in `1662`. WASM lint remains to run. Master-targeted
Actions will also be required after retargeting.

Meanwhile isolated the independent CSV preparation work in
`.worktrees/csv-batch-transaction-boundary`, branch
`fix/csv-batch-transaction-boundary`, based on `02f7b58`. Both parser/test files
match `e5fcc5a`; the only additional source hunk is the exact generic
`jobs::cancel` from `4fac6af`, needed by the original cancellation barrier test.
No other import admission/status commands were copied and no test was weakened.
Formatting passed (`26556`); runtime/cache/lint gates are not yet accepted.
Do not run concurrent local Cargo checks in the shared target; start its suite
after #228's server/WASM checks finish. #226's full check remains live in `46319`.

### 2026-10-06 — Resume extraction verification

The preceding response restated a goal rather than advancing repository state
(no progress). Read the actual saved objective and revalidated #216 as merged
at `02f7b58`. Existing extraction branches and unpublished changes remain in
place. Both original Nix handles (`46319`, `1662`) were confirmed live and
resumed, not restarted. #226 has passed SQLx preparation and its test derivation;
the overall check remains pending. #228's client release build has passed.

#228's offline all-target workspace/server Clippy passed (`17183`), followed by
WASM Clippy (`40125`); neither required source changes. Its full Nix check and
master-targeted Actions after retargeting remain acceptance gates.

Rechecked the CSV extraction against `e5fcc5a`: both original source/test blobs
are unchanged. The sole `jobs.rs` adaptation is the original executor-generic
cancellation helper, keeping its SQL predicates unchanged and allowing the
original test to cancel through its held publication barrier. Reviewed the
preparation loop, first/subsequent batch boundaries, preview rollback/resume,
parser shutdown and unchanged pool callers. No high/critical source finding in
this bounded extraction; runtime verification remains pending. Started the full
workspace suite on a fresh private PostgreSQL cluster after #228's local lint
finished; no concurrent local Cargo command uses the shared target.

Next: collect the CSV suite, regenerate its complete SQLx cache, verify both
offline targets and publish only after these gates pass. Collect both live Nix
checks. Many original source/specification groups still require extraction and
reconciliation; the separation goal is not complete.

#226's original full Nix check completed successfully (`46319`, exit 0,
`all checks passed!`), including browser, SQLx, tests and NixOS end-to-end
checks on the compatible host system. No restart was required. A single remote
query confirmed current-head GitHub Flake Check and Format success for #223,
#224 and #225; their Nixbot builds remain in progress. #227's Format passed
and Flake Check remains running. These results do not waive master-targeted CI
for stacked PRs after retargeting.

Isolated the next writer boundary in `.worktrees/invoice-write-authority`,
branch `fix/invoice-write-authority`, on #228 `0e1e675`. It contains exactly the
five Rust/test changes from `a25e544`, not its earlier financial-reader changes.
The new 564-line test file matches original blob `36ca710` byte-for-byte.
Existing #216 invoice client filtering and #228 coordination tests remain.
Formatting passed unchanged (`41998`); no runtime/cache acceptance yet.
The real dependencies are #228's organization helper and #227's actor-aware
user-role command. Original T192–T194 retain separate reader, source/fee/entry
order and full T042 gates. Verify this boundary after the CSV local checks;
do not replace its real opposing commands with fixture writes.

Further signature tracing found an additional genuine invoice dependency:
`a25e544`'s unchanged tests call `preview::prepare` with `(org_id, actor_id)`,
introduced by `22ffdab`. Master/#228 still accepts only `org_id`. Therefore
the isolated writer patch is not yet a compilable delivery and must not be
published as ready or tested by dropping the actor argument. Preserve it in
the new worktree until the financial snapshot consumer extraction and its
shared #220 helper are integrated explicitly. `editing::load/review` also have
separately retained `15c82ef` snapshot changes; do not copy whole invoice files
and overwrite #216 filtering. CSV verification remains independent.

The CSV workspace suite passed (`95407`): 821 app, 180 integration and 121 core
tests, 1,122 total, with 11 existing manual tests ignored. Complete SQLx
regeneration is running in `55336` on a fresh private PostgreSQL cluster.

CSV SQLx regeneration passed (`55336`): 1,003 base descriptors unchanged, two
additions matching `e5fcc5a` byte-for-byte, 1,005 total. Both new queries belong
to the preserved transaction-observation/publication-barrier tests. Unsigned
commit `e9898ed` contains exactly three Rust files and those two descriptors.
Offline all-target workspace/server Clippy is running in `17809`; WASM lint
and publication remain next. #228's original full Nix check is still live in
`1662`, without a restart. The unfinished invoice worktree is preserved and
must wait for its genuine financial-preview dependency before runtime checks.

Offline server Clippy passed (`17809`), followed by WASM Clippy (`15392`), with
warnings/performance lints denied and no source changes. Published commit
`e9898edbbe53d64a16ab77f9bca9961ff520a4a8` as independent draft #231 on master
(five files, 257 insertions and 10 deletions). Required current-head CI is
running in [37490375038](https://github.com/numtide/horae/actions/runs/37490375038).
The next safe source boundary is the retained financial snapshot consumers from
`22ffdab`/`15c82ef`, reusing #220's helper and preserving #216 queries. Resolve
their dependencies before completing the staged invoice writer extraction.
Keep #228's verified-live Nix handle `1662`; no merge or original-PR closure is
authorized or performed. Newly observed #229/#230 concern Darwin CI/platform
support, not this extraction; they remain untouched.

### 2026-10-06 — Financial snapshot consumer extraction

The preceding turn made progress: #231 was published with all local gates
passed and the ledger was pushed. Reconfirmed #216's merge, read the saved
objective and resumed #228's original Nix handle `1662`; server/client release
builds passed and Clippy continues. No process was restarted.

Created `fix/financial-snapshot-authority` in
`.worktrees/financial-snapshot-authority` on #220 `bc0c7a0`. Extracted the
consumer/test hunks of `22ffdab` and `15c82ef` together, reusing #220's existing
manager-snapshot helper without copying or changing it. This covers original
T098–T103: project fee balances, invoice preview and invoice editor load/review.
The original HTTP matrix is wired into the existing real-cookie harness beside
the retained legacy-reader checks. #216 invoice filtering remains intact;
there is no schema, UI, writer-policy or dependency change.

Necessary fixture adaptations avoid importing unrelated permission storage or
project writer commands: existing organization-gate helpers are expanded to
their exact `FOR NO KEY UPDATE` SQL, as already done by #220's reader tests.
The original revision-zero assertion becomes an unchanged PostgreSQL `xmin`
assertion before/after actor revocation (the organization tuple must not change
at all). The separate organization-revision refresh case uses `SET name=name`
to create a new tuple version without changing business values. Both cases
still observe real blocked readers, require fresh authorization/business
snapshots and retain all original assertions about denied access, amounts,
invoice revisions, cancellation and business-row preservation. Test names
describe tuple changes rather than a permission column absent from master.
The canonical revision-specific originals remain preserved for later integration.

Formatting passed (`8739`, only the two adapted test files changed). The full
workspace suite is running in `49719` on a fresh private PostgreSQL cluster;
core/app/test entrypoints were touched to avoid stale shared-target artifacts,
without content changes. No other local Cargo check shares that target.
Next: review exact source/fixture differences, collect the suite, regenerate
SQLx, verify offline server/WASM and publish with #220 as its real dependency.
The prepared invoice writer worktree remains untouched until this dependency
can be composed without duplicating PR contents. The overall goal is incomplete.

The full suite passed (`49719`): 840 app, 180 integration and 121 core tests,
1,141 total, with 11 existing manual tests ignored. All 14 added snapshot tests
and the extended registered-session matrix ran. Preview, editor implementation
and fee-test files match `15c82ef` exactly; HTTP matrix/root snapshot tests also
match their original blobs. The only original invoice-test block absent here is
the assignment/FK regression already delivered in #228, not a removed test.
Source review confirmed unchanged financial helpers/queries, all four current
session identities, same-tenant resource predicates, no mutation guards changed
and materialization/commit before delivery. No critical/high finding within this
bounded extraction. Complete SQLx regeneration is the next running gate.

SQLx regeneration passed (`29604`): all 1,021 #220 descriptors unchanged, 24
additions (22 match `15c82ef`, two describe the documented `xmin`/no-op tuple
fixtures), 1,045 total. Saved unsigned financial commit `0bb5721`; offline
server Clippy runs in `85779`, full Nix checks on that committed head in `80170`.

Resolved the invoice writer's multiple prerequisites without modifying or
retargeting their PRs: `.worktrees/invoice-authority-prerequisites`, branch
`integration/invoice-authority-prerequisites`, starts at #228 `0e1e675` (which
includes #227) and replays #220 as `41e595d` and the financial reader extraction
as `0046dad`. Existing identical SQLx descriptors are shared rather than
duplicated. This branch is an integration/review base, not a separate delivery
or a request to merge another PR. Its source must be verified together with the
writer, and eventual master retargeting still requires all prerequisite PRs.

Saved the prepared five-file writer patch as `a428015`, backed it up at
`backup/invoice-write-extraction-before-integration`, then rebased only the new
unpublished `fix/invoice-write-authority` onto that integration base (`680407c`).
Its diff still consists of the five original `a25e544` source/test changes and
the 564-line test blob remains exactly `36ca710`. Repository `rebase.updateRefs`
also moved the new backup automatically; restored it immediately to `a428015`
with a compare-and-swap ref update. Original #212/#217 backup refs were checked
unchanged. Future extraction rebases must explicitly use `--no-update-refs`.
No PR was merged and no real data was changed. Run writer integration tests
after the financial reader local checks finish; do not run shared Cargo targets
concurrently. Its additional SQLx descriptors are not prepared yet.

#232 is published on #220 with clean head `0bb5721`. Offline server Clippy
passed (`85779`), followed by WASM (`86132`). Full Nix remains live in `80170`;
#228's original `1662` progressed through browser checks into test builds.
`git range-diff` confirms the invoice writer's rebased `680407c` is patch-equal
to saved `a428015`; its preview/snapshot sources are unchanged from #232 and its
editor/test roots match original `a25e544`. Full combined workspace verification
is now running in `37438` on private PostgreSQL. Next: collect that suite,
regenerate its cache, run both offline target lints and publish the writer only
with explicit prerequisite/retargeting instructions. Keep all live checks;
do not duplicate their runs or claim the remaining original groups accounted.

### 2026-10-06 — Invoice writer integration verification

The immediately preceding conversational turn only supplied a proposed goal
prompt (no authoritative progress). Re-read the active saved objective and
revalidated the actual worktrees and process handles instead of restarting
work. #216 is confirmed merged at `02f7b58`; the original worktrees remain
preserved. The earlier extraction work did make progress as recorded above.

The combined writer suite `37438` completed successfully on `680407c`: 861
app, 180 integration and 121 core tests, 1,162 total, with 11 existing manual
tests ignored. Formatting `92986` already passed unchanged. Complete SQLx
regeneration now runs in `28714` on another fresh private PostgreSQL cluster;
do not interpret its temporary cache removals as a final diff before it exits.

#228's original full Nix handle `1662` exited successfully with `all checks passed!`, including browser, SQLx and the NixOS end-to-end test; incompatible
platforms were omitted as reported by Nix. #232's `80170` is still live and
has completed its client release build. No check was restarted or bypassed.
Next: compare the writer's final SQLx cache with its integration base, run
offline server/WASM lints, publish the narrowly scoped draft with its explicit
prerequisites, and retain the single ledger as the accounting source.

Writer SQLx preparation passed (`28714`): 1,069 integration-base descriptors
unchanged and eight additions, all byte-identical to `a25e544`, 1,077 total.
Committed only those descriptors as unsigned `8a6cb2a`; source/tests remain
unchanged from the fully tested `680407c`. Offline all-target server Clippy
passed (`8831`). Full Nix runs on the clean committed head in `1215`; WASM
Clippy follows server lint without sharing concurrent Cargo work.
The source review was rechecked against T192–T194 and the original invoice
contract: current same-tenant actor before replay/write, common organization
and invoice serialization prefix, test-only actorless adapters remain test-only,
and events remain outside committed transactions. No broader policy or full
resource-lock hierarchy acceptance is claimed.

WASM Clippy passed (`75047`). Published the prerequisite review base `0046dad`
and writer `8a6cb2a`, then opened draft [#233](https://github.com/numtide/horae/pull/233).
Its diff is 13 files, 811 insertions and eight deletions: five original Rust/test
changes plus eight query descriptors. The body explicitly prohibits merging
into the integration base; first deliver #227 → #228 and #220 → #232, then
retarget/rebase the writer-only change onto master and run current-head CI.
No prerequisite branch was changed. #228's PR body now records its successful
full Nix check. Original #212/#217 backup refs and all 18 unfinished local paths
were rechecked unchanged; the writer's own pre-integration backup is `a428015`.

Next-boundary inspection found that extracting only the first profile command
commit `f5e0dde` would omit later actor/target/last-admin SHARE locks and bounded
READ WRITE transactions from `03e90b1`. Preserve those repairs with the commands.
Final `profiles.rs` also uses the shared `ProfileDraft`/command DTOs and the
historical `RemovedRelationship` type (the latter moved in `8db19ba`); it does
not require activating the editor UI. Final profile tests add unrelated child
modules after `03e90b1`; separate their module wiring without dropping original
profile cases. The next extraction must resolve #226 plus #221 prerequisites,
retain migration 0044 and current profile behavior, and account separately for
later editor/subject/directory tests. This is inspected dependency evidence,
not a completed extraction. Keep Nix handles `80170` (#232) and `1215` (#233)
without restarting them. The overall original-change reconciliation is incomplete.

Remote verification confirms #233 is draft on the intended review base, head
`8a6cb2a`, with exactly the 13-file writer diff. #227's unchanged `142eda1` now
has successful GitHub Flake Check and Format in run `37486457726`; Nixbot build
is still pending. #231's `e9898ed` remains on the original live Flake Check run
`37490375038` with Format passed; no rerun or repeated polling was requested.

### 2026-10-06 — Person-profile command extraction

The preceding iteration made progress: #233 was published, its combined tests
and offline checks passed, and current #227/#228 evidence was recorded and
pushed. Re-read the saved objective and confirmed #216 remains merged before
editing. Existing Nix handles `80170` and `1215` were both verified live; no
restarts. No original branch or unfinished work was changed.

Created `integration/profile-command-prerequisites` in its isolated worktree
at `46f02f7`: #226 `82d15f3` plus the exact #221 domain patch, replayed without
conflicts. This is a review/test base, not a separate delivery or merge target.
Created `refactor/person-profile-commands` in `.worktrees/person-profile-commands`
on that base. It carries original migration 0044, byte-identical final
`profiles.rs` from `db3935d`, and all 27 profile-command PostgreSQL tests from
`03e90b1`. The only removed test-file lines wire the separately retained editor
child module; no profile assertion or helper was changed. Later additions to
this test root only wire other consumer modules and remain separately accounted.

The four needed profile DTOs and historical `RemovedRelationship` retain their
exact original fields and serde attributes. They extend #226's existing
server-only command models without including unrelated UI/audit reader DTOs.
Template commands/tests and their shared transaction prelude are unchanged.
This preserves the later actor/target/survivor SHARE locks, bounded local
transaction settings, strict identity and replay checks, exact relationship
confirmation, audit rollback and last-administrator protection. Commands stay
internal and refuse policies 0/future; no endpoint, UI, backfill or activation.

Read the current person-profile and editor contracts and original T056–T058 /
T126–T129 task hunks. This extraction covers the internal command boundary and
its later hardening, not the separately retained editor/session/UI acceptance.
The combined core permission modules match the final original exactly.
Formatting passed unchanged (`1025`); the complete workspace suite is running
in `7729` on private PostgreSQL. A separate disposable migration-upgrade check
`59932` starts from the prerequisite schema including 0047 and then applies the
retained 0044, to verify actual delivery order without renumbering migrations.
Next: collect both checks, regenerate SQLx, run offline server/WASM gates,
review/publish the bounded draft and continue the remaining original accounting.

The prerequisite-schema upgrade passed (`59932`), applying only original
migration 0044 after the already-applied 0047. All four command/draft DTO blocks
were compared byte-for-byte with the original and match. Template implementation
and test files also match final `db3935d`; no substitute fixture commands are used.
Saved the seven-file extraction as unsigned `9005b1b` (1,744 insertions, two
deletions); cache generation and publication remain pending, not claimed ready.
Source review found no critical/high issue within this inactive internal boundary;
runtime verification remains mandatory. The full suite `7729` is still live in
compilation; no duplicate local Cargo run has been started. #232's existing Nix
run progressed through Clippy into browser checks, while #233 completed its SQLx
derivation and client build; both full checks remain pending.

Read-only inspection for a later import-authority extraction confirms `d7a5a21`
requires the existing organization lock helper and threads the authenticated
actor through connect/disconnect/account-switch commits, including original
credential tests. `4fac6af` and `b4672a4` add command/status/download authority;
later `482b7c5` adds requester provenance and has a separate storage dependency.
Do not copy final import files blindly across those boundaries or duplicate
the executor-based cancel change already extracted in #231.

The full combined workspace suite passed (`7729`) on the unchanged `9005b1b`
source: 876 app, 180 integration and 189 core tests, 1,245 total, with 11
existing manual tests ignored. All 27 original profile tests ran alongside the
template and domain prerequisites. The initial compilation took 4m25s and the
app tests 120s; long-running notices were not failures or a reason to restart.
Full SQLx preparation now runs in `56264` on another private temporary database.
Next: compare the final cache with the prerequisite base and original descriptors,
then serialize offline server/WASM lint before publishing the draft.

SQLx preparation passed (`56264`): all 1,063 prerequisite descriptors unchanged,
35 additions byte-identical to original `db3935d`, 1,098 total. Offline all-target
workspace/server Clippy is running in `89503`; WASM must follow it sequentially.
No source or test repair was needed after the full suite. Original migration
checksums and prerequisite source remain intact.

Published draft [#234](https://github.com/numtide/horae/pull/234) at `45d219e`
against the explicitly documented review base `46f02f7`. Verified remote diff:
42 files, 2,437 insertions and two deletions (seven source/schema/test paths plus
35 query descriptors). Server Clippy passed (`89503`); WASM runs in `13136`.
Full Nix runs on the clean committed head in `41624`. No new source changes
followed the successful suite; the second commit only adds its verified cache.
The PR requires #219, #221/#222 and #226 on master before rebase/retarget and
fresh required CI. It must not merge into its integration review base. No merge,
original-PR closure or policy activation was performed; remaining consumer and
specification accounting is still incomplete.

WASM Clippy passed (`13136`) with no code changes. #234 now has all local
suite/cache/offline-lint/format gates passed; full Nix remains in `41624`.
Next: collect existing Nix handles `80170` / `1215` / `41624` without reruns,
then continue the retained import-authority boundary from its actual source
dependencies, preserving #231's already-delivered cancellation adapter.

### 2026-10-06 — Harvest connection authority extraction

The intervening conversational turn drafted a goal but made no repository
progress. Re-read the active saved objective and resumed the existing ledger;
the last implementation iteration delivered #234. Confirmed #216 merged at
`02f7b58` before editing, and verified all three existing Nix handles live
(`80170`, `1215`, `41624`); none was restarted. #234's Nix SQLx derivation and
client release build have now passed, but its full check is not yet terminal.

Created `fix/harvest-connection-authority` in its isolated worktree on #228
`0e1e675`. This dependency supplies the organization SHARE helper and coordinated
legacy access writers; no duplicate helper or new integration branch is needed.
The eight Rust/test files match original `d7a5a21` byte-for-byte, including the
seven production-writer authority tests (`afc063dd8c8a64668475a4865a813619b6bffff4`).
Read original T074–T076 and the connection-management contract. Later changes to
the credential writer, account switch and this test module are absent; later
job/streaming work in `harvest.rs` remains separately retained.

The extracted path obtains the actor only from the authenticated wrapper or
validated OAuth attempt, reserves the import nonblockingly, then checks the
organization and active same-tenant Administrator under READ COMMITTED/SHARE
before the existing generation gate. Authority remains locked through commit.
External OAuth exchange stays outside that transaction; account binding,
generation checks, encryption, import history and watermarks are unchanged.
Safe forbidden projections cover both callback and server-function errors.
No new policy activation, schema change, UI or external Harvest mutation.

Adversarial source review covers admission-to-write races, actor-only waits,
inherited REPEATABLE READ, missing/foreign/inactive actors, writer-first retention,
rollback and reservation cleanup. No critical/high issue found within this
bounded extraction; tests remain necessary evidence. Formatting passed unchanged
(`43759`). The complete workspace suite runs in `5611` using private PostgreSQL;
next regenerate SQLx, run offline server/WASM checks, then publish the bounded
draft. Full original-change and specification reconciliation remains incomplete.

Saved the exact source extraction as unsigned `fcffd12`; its stable patch ID
matches the original Rust delta (`1d297236d8c4fa3a0abdd49c41ec2d53e682470a`).
The full suite has finished compilation and is running tests. Original tracked
edits still compare equal to the saved uncommitted snapshot, and all six untracked
files compare equal to the recovery archive. #231's existing CI run `37490375038`
now reports successful Flake Check and Format on unchanged `e9898ed`; its Nixbot
build remains in progress. No rerun, merge or closure was requested.

Read-only next-block inspection: `4fac6af` introduces the importer command
transaction plus executor-based queue helpers, and `b4672a4` shares its authority
guard with paged error downloads. They form a cohesive command/result boundary
on the same organization helper. Later `482b7c5` only extends these callers for
requester attribution and migration 0045's `(org_id, id)` user foreign key,
which needs the #222 schema prerequisite. Preserve that distinction rather
than silently carrying storage or activating worker policy. The cancel adapter
already owned by #231 must be accounted as an identical shared prerequisite,
not a second independent delivery of that change.

The complete combined suite passed (`5611`): 845 app, 180 integration and 121
core tests, 1,146 total, with 11 existing manual tests ignored. This exercises
all seven retained authority tests and the #227/#228 prerequisite behaviors
together. No source/test adaptation was needed. Full SQLx regeneration is next,
then offline server/WASM lints and publication; old results are not used to claim
these remaining checks passed.

Full SQLx preparation passed (`9128`): 1,030 prerequisite descriptors unchanged,
ten additions byte-identical to `d7a5a21`, and only the superseded post-HTTP
authority query removed, for 1,040 total. Two added descriptors already existed
earlier in the original history (session REPEATABLE READ and transaction READ
COMMITTED), explaining the difference from that original commit's eight additions.
Saved the cache as unsigned `2fcecd1`; source remains identical to tested
`fcffd12`. Final bounded diff: 19 files, 739 insertions and 49 deletions (eight
Rust/test paths plus eleven cache changes). Offline server lint runs in `91709`;
full Nix runs on clean head `2fcecd1` in `72580`. WASM follows server lint.

Published draft [#235](https://github.com/numtide/horae/pull/235), head `2fcecd1`,
base `fix/project-access-lock-order`. Its description includes the exact source
provenance, tests, pending checks and #227 → #228 → #235 integration order.
No migration, runtime policy activation, merge or original-PR closure. Complete
this extraction's remaining lints before editing the next command/result block;
retain all existing Nix handles without restarting them.

#232's original full Nix check (`80170`) completed successfully on unchanged
`0bb5721` with `all checks passed!`, including browser, SQLx and NixOS end-to-end
checks; incompatible systems were explicitly omitted by Nix. Its PR description
now records that result. #235's offline server/all-target lint also passed
(`91709`); WASM runs in `21947`. #233's original full Nix check (`1215`) passed
its server release build and VM e2e script and continues its browser matrix;
#234 (`41624`) and #235 (`72580`) remain live full checks, not completed claims.

#235's WASM lint passed (`21947`) without edits. Its published draft now records
all local suite/cache/offline-lint/format checks passed, with full Nix pending.
Next: retain `1215` / `41624` / `72580` until terminal, and extract the original
`4fac6af` + `b4672a4` job command/result boundary after resolving the already-owned
#231 cancel adapter and actual test/base dependencies. No original work is lost
or relabeled complete; broad original-code and specification accounting remains
unfinished, so the overall goal remains active.

### 2026-10-06 — Import job commands and bounded downloads

The preceding iteration made progress: #235 was published, its local gates
passed, and #232's full Nix result was recorded. Re-read the saved goal, verified
#216 remains merged, and resumed the three existing Nix handles (`1215`, `41624`,
`72580`) confirmed live. Repository instructions and the constitution are unchanged.

Created isolated review base `integration/import-job-authority-prerequisites`
at `26d6159`: #235 `2fcecd1` plus exact #231 `e9898ed`, replayed without conflict.
The #231 executor-based cancel adapter is therefore inherited, not duplicated
in this extraction. No prerequisite branch was changed. This integration base
is not a delivery PR or merge target; #227 → #228 → #235 and #231 must reach
master before rebasing/retargeting the bounded command/result diff and fresh CI.

Created `fix/import-job-authority` in its own worktree on that base. The eight
Rust/test files match original `b4672a4` byte-for-byte: both `4fac6af` command
authority and the subsequent shared download guard are preserved together.
Read T077–T082 and both transaction contracts. The six command/status helpers
use trusted actor IDs and retain current same-tenant Administrator authority
through the existing generation/job/upload operations and returned projection.
Upload buffering precedes authorization locks. Duplicate/no-op paths still
authorize; generation, payload, retry, upload retention and queue semantics remain.

The shared guard also authorizes initial report preparation, each bounded page
of up to 16 fragments and the captured inline/empty tail. Transactions commit
before client-paced output. Already authorized buffered bytes may drain, but
revocation aborts the next page/tail instead of producing successful truncated EOF.
Snapshot boundaries, exact retained errors, headers and missing-fragment failures
remain unchanged. Original HTTP tests revoke access after endpoint admission
and before upload acceptance/first body consumption. Test-only queue adapters
remain test-only; no worker policy, requester migration, UI or cutover is included.

The final-original guard, download implementation and stream tests have no later
changes; the command test root only later adds the separately retained requester
module. Reviewed production callers, tenant/error projections, transaction and
connection lifetimes, cancellation/retry, wait ordering and prerequisite ownership.
No critical/high source finding in this bounded extraction; runtime evidence is
still required. Formatting passed unchanged (`2345`); full combined workspace
tests run on private PostgreSQL in `48997`. Next collect the suite, regenerate
SQLx, run offline server/WASM gates and publish with the real dependency order.

Saved the eight-file extraction as unsigned `c384d48` (1,519 insertions,
111 deletions); all eight hashes match `b4672a4`. The inherited CSV preparation,
connection authority and account-switch files remain unchanged from the review
base. No new lower-level queue bypass or worker grant was introduced.

Read-only inspection of the next original increment confirms `482b7c5` has five
requester tests, a four-line migration 0045 and focused command/HTTP/historical
report adaptations. It needs migration 0042's `users_org_id_id_key`, supplied
by #222, in addition to these command helpers. Both the requester test module
and migration remain unchanged at final `db3935d`. Preserve their historical
NULL and duplicate-first-author rules, private DTOs and NO ACTION tenant FK;
this is attribution only, not authority to execute jobs for a revoked requester.
An eventual extraction must test the actual prerequisite-schema upgrade order,
including already-delivered migration 0047, without renumbering original migrations.

### 2026-10-06 — Command/download verification and publication

The preceding user-facing turn only supplied the requested goal text (no
implementation progress); the existing extraction remained preserved. Re-read
the attached objective and repository instructions, confirmed #216 merged at
`02f7b58`, and resumed existing verification handles rather than restarting them.
The command/download suite (`48997`) finished successfully: 862 app, 180
integration and 121 core tests, 1,163 total, with 11 existing manual tests ignored.
All eight source/test files remain at `c384d48`. Complete SQLx regeneration runs
in `73391` against private PostgreSQL; offline server/WASM lints follow.

The previously observed terminal result for #233's full Nix run (`1215`) was
successful on `8a6cb2a`, including browser and NixOS checks; incompatible systems
were omitted. Recorded this in the delivery table and PR description. Its original
handle is now closed, not a reason to restart the completed check. #234 (`41624`)
and #235 (`72580`) were confirmed still live; neither is counted as complete.

Full SQLx regeneration passed (`73391`): all 1,042 inherited descriptors unchanged
plus eight additions byte-identical to `b4672a4`, 1,050 total. Unsigned cache commit
`95bdf4a` leaves the eight source/test files unchanged. Published draft
[#236](https://github.com/numtide/horae/pull/236) on review base `26d6159`:
16 files, 1,632 insertions and 111 deletions. All added lines are original code,
tests or regenerated cache, not new functionality. Offline server lint runs in
`96055`; WASM follows. Full Nix runs on clean `95bdf4a` in `65754`.

Rechecked preservation: original tracked work matches saved snapshot
`backup/pr212-split-20261006-uncommitted` and the untracked archive comparison
passed unchanged. The next attribution increment requires #222 (and its #219
base) plus #236; original migration 0045 relies on 0042's composite user key.
Keep its five original requester tests, HTTP forgery checks and legacy report
adaptations together. Test the actual upgrade from the combined base already
containing 0047; no original migration renumbering, backfill or worker-policy grant.

Offline server lint (`96055`) and WASM lint (`36645`) both passed on unchanged
#236 source/cache. Its PR description records all local gates passed and full
Nix still running (`65754`). Remote inspection confirms draft head `95bdf4a`,
the intended review base and exactly the bounded 16-file diff.

#234's original full Nix run (`41624`) completed with `all checks passed!` on
unchanged `45d219e`, including browser and NixOS e2e checks; incompatible systems
were omitted. Recorded the result without restarting the successful run.

Created isolated review base `integration/import-requester-prerequisites` at
`cfb8240`: #236 `95bdf4a` plus exact #219 `ec7ddbd` and #222 `e9695fd`, replayed
without conflict. Created `feat/import-job-requester` in its own worktree and
extracted only the seven source/schema/test paths from original `482b7c5`.
Unsigned source commit `e0293d7` has 409 insertions and 14 deletions; stable patch
ID `36ce831fe633f5e8882627d7785cbf49f501c28c` equals that original filtered patch.
The report-test hunk is preserved without duplicating #223's separate lock-order
tests. Migration 0045 and all five requester test bodies remain unchanged.

Formatting passed unchanged (`77129`). Combined full workspace tests run in
`49490` on private PostgreSQL. A separate private-database migration-order check
(`73733`) applies the actual prerequisite schema including 0047, then 0045, and
repeats migration execution. It passed with the exact applied set 0042/0045/0047;
the populated historical-state fixture remains covered by the running Rust suite.
Next: collect the suite result, regenerate requester SQLx, run offline lints and
full Nix, then publish its bounded draft with both prerequisite chains. Continue
the existing `72580` / `65754` Nix handles; no restart or merge is authorized.

### 2026-10-06 — Requester verification and next independent boundary

The preceding iteration made progress: #236 was published with passing local
gates, #234's full Nix result was recorded, and the original requester increment
was extracted with its migration-order check passed. Re-read the attached goal,
confirmed #216 merged, and verified repository/skill instructions unchanged.
The three existing verification handles (`49490`, `72580`, `65754`) remain live;
no process was restarted because of quiet compilation. Published review base
`cfb8240` unchanged. Requester suite compilation finished in 4m28s and began the
889-test app binary; completion is not yet claimed.

Read-only next-boundary inventory: `0793ce7` changes four notification source/test
paths plus cache/specification material, with 13 authority tests. Those four
paths have no later original changes through `db3935d`; the two pre-existing
notification files match master before that commit. The preparation contract
preserves the existing recipient predicate, bounded send, attempts and stable
message identity; OP37 canonical mapping and enqueue integration remain open.
Inspect the complete implementation and test dependencies before extracting.
This looks independent of the permission-schema chains, but no completed review
or runtime acceptance is claimed. Use disposable databases and executable local
sender stubs only; never send real budget mail during verification.

Requester suite `49490` passed: 878 app, 180 integration and 160 core tests,
1,218 total, with 11 existing manual tests ignored. This includes all five
original requester cases, HTTP forged-identity assertions and the populated
historical migration fixture on the combined prerequisite code. No source or
test changes were needed. Complete SQLx regeneration runs in `92846`; offline
server/WASM lints must finish before reusing the shared local Cargo target.

Read the complete notification preparation/delivery implementation, 13 new
authority tests, existing sender stub/lifecycle tests, outbox claim/acknowledgement
helpers and migration 0039's parent revision triggers. Actual prerequisites are
already on master: there is no permission-state storage, new grant, actor helper
or external transport dependency to import. Tests use only local executable
stubs and temporary databases. The transaction takes organization, recipient,
project then outbox locks, rechecks stored claim/payload/attempt/clock and current
eligibility in fresh statements, and commits before external delivery. Retargeted
or replaced claims skip without corrupting replacement state; terminal rejection
stays under the claim lock. No critical/high source finding in this bounded
review; full runtime evidence remains necessary, and post-release recall or
exactly-once transport is not claimed.

Created independent branch/worktree `fix/budget-email-authority` /
`.worktrees/budget-email-authority` on `02f7b58`. Unsigned `bf22776` contains four
source/test files, 1,015 insertions and 49 deletions. All four file blobs match
`0793ce7` (and final `db3935d`) exactly; stable patch ID
`48c6efde7e99c4a446dfb2991f1a40563a7d6385` matches the original filtered patch.
Formatting passed unchanged (`29642`). Do not run its local Cargo suite until
the requester SQLx and offline gates release the shared target. No real mail,
new schema, worker activation or original branch changes occurred.

Requester SQLx preparation passed (`92846`): 1,065 prerequisite descriptors
unchanged, thirteen additions byte-identical to `482b7c5`, and four superseded
descriptors removed, 1,078 total. The entire regenerated cache patch has the
same stable ID as the original (`1f6cf80b1a0b38424bf0f7aa109e6eedffc157af`).
Saved unsigned cache commit `2242361` and published draft
[#237](https://github.com/numtide/horae/pull/237) on review base `cfb8240`.
Bounded diff: 22 files, 629 insertions and 75 deletions. The original source and
five requester tests are unchanged. Offline server lint runs in `16577`; WASM
follows. Full Nix runs on clean `2242361` in `63735`. No merge into the review
base is authorized; deliver both prerequisite chains, then retarget and rerun CI.

#235's existing Nix run (`72580`) completed its server release build and advanced
to browser/NixOS checks. #236 (`65754`) completed its Nix test derivation and is
still running remaining checks. Neither full result is yet claimed successful.

Requester offline server lint (`16577`) and WASM lint (`11771`) passed unchanged.
#237 now records all local gates passed and full Nix (`63735`) running on the
published head. Remote verification confirms `2242361`, base `cfb8240`, draft
status and the bounded 22-file diff. The original tracked snapshot and untracked
archive were compared again and remain intact.

With the shared local target released, started the independent budget-email
workspace suite in `57086` using the existing private-PostgreSQL wrapper and
original local sender stubs. Next collect this suite, regenerate its complete
SQLx cache, run offline server/WASM lints, full Nix and publish the independent
draft. Preserve existing Nix sessions `72580` (#235), `65754` (#236) and `63735`
(#237) until terminal. Specification reconciliation and many other original
behavior groups remain unassigned; the overall goal is not complete.

### 2026-10-06 — Notification checks and bounded artifact analysis

The preceding iteration made progress: #237 was published with all local gates
passed, and the independent notification source/tests were extracted at `bf22776`.
Re-read the attached goal and confirmed all four existing verification handles
live. #216 remains merged. Budget-email suite `57086` compiled in 4m06s and is
running its 843-test app binary; no suite completion is claimed yet.

Master advanced externally to `1b8fa4f` through #230. Inspection shows only
`flake.nix` changed: Linux remains the checked package systems, with ARM Darwin
retained for local development. No application, schema or dependency lock changed.
Do not modify the worktree underneath a live verification process. After its
current suite finishes, rebase only the unpublished notification extraction onto
this master using `--no-update-refs`, verify patch preservation, and run current-head
checks. Keep old-head evidence labelled; do not silently certify a rebased head.

Ran the actual Spec Kit analyze prerequisite command once in the original
worktree, resolving feature 015 without overrides. No before/after hooks exist.
Read constitution 1.1.0 and the relevant spec stories/requirements, plan gates,
T086–T088/T114–T116 and their previously reviewed contracts. Read-only analysis
covers six requirement IDs (FR-006/007/010/017/018 and SC-006) and six tasks, all
with traceable bounded coverage, not full-feature acceptance. No new constitutional
conflict or high/critical finding was identified within these two increments.
Retained findings: medium stale T161 prose contradicts its closed checkbox and
recorded later evidence; low distinction needed between historical T113→T114
work order and the independently verified notification release dependency.
No feature artifact or constitution was modified during analysis.

Documentation preservation must use the original `9301112..db3935d` change set,
not a wholesale old-tree replacement over current master. Direct old-tree versus
master comparison misleadingly lists newer Clients/parity specs as deletions;
those are upstream additions and must remain. Governance amendment, committed
feature artifacts and unfinished local Client contract changes remain separate
accounting work, not permission to declare their underlying features complete.

### 2026-10-06 — Notification rebase and completed connection checks

The intervening response only restated the requested goal and made no repository
progress. Revalidated the four existing process handles rather than restarting
them, and confirmed #216 remains merged at `02f7b58`.

Notification suite `57086` passed on old head `bf22776`: 832 app, 180 integration
and 121 core tests, 1,133 passed with 11 existing manual tests ignored. Rebased
only the unpublished extraction using `--no-update-refs` onto master `1b8fa4f`,
yielding unsigned `88149495025faecb059a0c4f13b5651918f4bec9`. The four source/test
blobs and stable source patch ID remain unchanged. Current-head suite `89138`
also passed all 1,133 tests; the same private-database session is now regenerating
the complete SQLx cache. Offline lints, full Nix and publication remain pending.

#235's original Nix process `72580` exited successfully on `2fcecd1`, including
the remaining browser matrix and NixOS checks. Updated its PR verification record.
This proves the local x86_64-linux gate for that head, not other architectures or
a future retargeted commit. #236 (`65754`) and #237 (`63735`) remain live; neither
is being restarted or counted as a completed full check.

Notification SQLx preparation in `89138` completed successfully. Regenerated
cache has 1,001 unchanged base descriptors, 54 byte-identical original additions
and two obsolete removals, 1,055 total. All original cache changes are represented;
ten additional descriptors already existed in the original parent through other
code, but are required by this standalone source/tests (transaction settings,
row locks and active-recipient fixtures). Therefore the entire cache patch ID
differs from `0793ce7`, while its contents are individually preserved. Saved
unsigned cache commit `7a7cede` and published independent draft
[#238](https://github.com/numtide/horae/pull/238), 58 files / 1,985 insertions /
53 deletions including generated metadata. Server offline lint `98267` and full
Nix `24069` run on the cache-inclusive head; WASM follows the local server lint.

Reviewed the next independent boundary: original `2abce9a`, tasks T021–T023.
Read the complete approval implementation, shared server helpers, four new
isolation tests and fixtures; traced every total/approve/reopen caller. The
three source paths and fixtures before this original commit match current master.
All external mutation routes still require the session's active Manager/Admin;
the helper's organization comes from that trusted user. Only tenant-local
submitted rows transition, only returned approval IDs select entries, foreign
reopen returns not-found, and invoiced entries remain untouched. Transactions
and post-commit plugin events retain their existing ordering. No new critical/high
finding within the tenant repair; this is not transactional revocation or the
future scoped/flexible approval policy.

Created isolated `fix/approval-tenant-isolation` /
`.worktrees/approval-tenant-isolation` on `1b8fa4f`, unsigned `d72b830`.
All three source/test blobs match `2abce9a` exactly, 248 insertions/31 deletions;
stable source patch ID is `3fad1ed12dbf8b71c61b579f56da244a39a7faf6` on both.
Formatting passed unchanged (`18944`). The new isolation test file is unchanged
through final `db3935d`. Later approval-name projection (`981d0e3`) and Timesheet
submission context (`b8b1c60`) remain separately preserved, not silently discarded
or folded into this invariant repair. Runtime suite/cache/lints/publication follow
once notification lints release the shared local target.

Recompared the original tracked backup and untracked archive: both remain intact.
No merges, original PR closures, production-data changes or real emails occurred.

Notification offline server lint (`98267`, 3m35s) and WASM lint (`43884`, 57s)
passed on cache-inclusive `7a7cede` without source changes. Updated #238's body;
remote inspection confirms draft status, master base, exact head
`7a7cede9d9ad8457e7bd78f049830d3144b468ac` and the bounded 58-file diff. Its full
Nix handle `24069` remains live, alongside #236 `65754` and #237 `63735`.

Released the shared local target and started approval isolation suite followed
by complete SQLx regeneration in `78215`, using the existing disposable-database
wrapper. Next: collect that run, verify cache provenance, run offline lints,
commit generated metadata unsigned, publish an independent draft and start its
clean-head Nix gate. Do not start another local Cargo process before `78215`
terminates. The original branches/backups and remaining specification/UI/domain
groups are not changed or declared complete by this progress.

### 2026-10-06 — Approval verification and identity-boundary inventory

The previous iteration made progress: #238 was published with local gates passed
and approval isolation was extracted without changing its original source/tests.
Re-read the goal, confirmed #216 merged and revalidated the four existing live
handles (`78215`, `65754`, `63735`, `24069`). No restarted builds or old-head
claims. Repository instructions and original working state remain unchanged.

Read-only next-boundary inventory groups `7f14e4b`, `981d0e3` and `3bb62ac` by
their shared identity-projection responsibility, rather than making three tiny
deliveries. The first introduces a legacy identity-only user-list DTO; the second
returns approval names in the authorized result instead of fetching a directory;
the third restricts the account-menu session DTO. The final user model, approval
model/page/UI tests and three HTTP test modules remain unchanged through `db3935d`.
The directory HTTP test must use its final version: `3bb62ac` replaces its former
expectation of sensitive own-user fields with the explicit session-identity
regressions. Do not restore those superseded expectations or drop the replacement
checks. Keep compatibility API financial projections outside this payload repair.

Current-master caller inventory includes Clients and invoice recovery added after
the original branch fork; preserve those consumers and compile them against the
narrowed identity. New Project's rate-bearing person DTO is a separate endpoint,
not permission to retain unnecessary rates in the directory response. Existing
registered-session test helpers are present on master: carry only the three
relevant modules/calls, not the original parent harness's unrelated canonical
authorization tests. This is dependency inventory, not a completed source review
or runtime acceptance, and no identity extraction has been created yet.

Approval isolation suite `78215` passed on `d72b830`: 823 app, 180 integration
and 121 core tests, 1,124 total, with 11 existing manual tests ignored. The four
original two-tenant helper regressions and the weekly behavior checks remain
unchanged. Complete SQLx preparation continues in the same private-DB session.

#236's original full Nix handle `65754` exited zero with all checks passed on
`95bdf4a`, including the completed browser matrix and NixOS e2e. Updated its PR
record. This is the local x86_64-linux result on the combined review base; it does
not replace future master-retargeted CI or authorize merging into that base.

Approval SQLx preparation completed (`78215`): 997 unchanged base descriptors,
eight byte-identical original additions and six superseded removals, 1,005 total.
The complete cache stable patch ID `260a95e0438066d47c0ac3aaa5d30ae13776a730`
matches original `2abce9a` exactly. Saved unsigned `66a256f8f2e8696e42d30eaca981b7e62dfbdfe4`
and published independent draft [#239](https://github.com/numtide/horae/pull/239),
11 files, 318 insertions/47 deletions including generated metadata. Offline
server then WASM Clippy run sequentially in `79148`; full clean-head Nix runs
independently in `87204`. No new schema or scoped approval activation.

#237's first full Nix process `63735` terminated with exit 1 after a successful
release build. Its browser derivation
`/nix/store/rahz5l50fgmd9qsmnazjwyzm2x71kh2j-horae-browser-checks.drv`
failed at `menu-popovers.cjs:204`: the last project-row menu remained visible
five seconds after decrementing its table's horizontal scroll offset. Read the
full failure log and the assertion/scroll handler. No pending request or browser
error was reported. The test, menu JavaScript/component and CSS are byte-identical
to #236, whose full browser suite just passed; requester code changes no UI path.
That comparison does not establish the root cause or turn the failed run green.
Updated #237's body and started one unchanged-head diagnostic full Nix rerun in
`30447`, preserving the first failure. Do not weaken the assertion or claim a
confirmed flaky cause. If reproduced, investigate with retained failure evidence
while continuing independent extractions; no unrelated UI repair is authorized.

Remote inspection confirms #239 is draft, targets master and contains exactly
`66a256f8f2e8696e42d30eaca981b7e62dfbdfe4` with the 11-file bounded diff.
Its combined lint process `79148` completed the offline workspace/server phase
successfully (3m13s) and advanced to WASM; do not count the combined process as
finished until that second phase exits. #237 diagnostic `30447` is rebuilding
only the three unfinished browser/VM derivations, reusing the successful compiled
package and other checks rather than restarting its release compilation.

#239's combined lint `79148` exited zero after WASM completed in 46.54s. Both
offline targets pass on published `66a256f`; the shared local Cargo target is
now free. Updated its PR body. #238's Nix test derivation completed successfully,
but full Nix `24069` remains live. #239 `87204` and #237's unchanged diagnostic
`30447` are also live. Keep the original #237 failure visible until the diagnostic
result is known; a partial log without that assertion is not proof of success.

Next independent work is the inventoried identity-projection group: complete
its source/consumer review before extraction, preserve current-master Clients
and invoice recovery, retain final HTTP/actual-page tests and avoid canonical
permission dependencies. No identity code has yet been changed. Mixed spec hunks,
governance and other UI/domain groups still require ownership and verification;
the overall separation goal remains incomplete.

### 2026-10-06 — Identity extraction and requester diagnostic result

The intervening response drafted the requested goal but changed no repository
state: classified as no progress. Re-read the attachment, confirmed #216 merged,
fetched unchanged master `1b8fa4f` and revalidated the three live Nix handles.
Tracked original backup and untracked archive comparisons both pass.

#237 diagnostic `30447` exited zero on unchanged `2242361`: all local
x86_64-linux Nix checks passed. Its browser log explicitly reports PASS for the
exact scroll/resize assertion that failed in `63735`. Updated the PR without
erasing the first failure or claiming a proven cause. Retargeted CI is still
required. #238 `24069` and #239 `87204` remain live; #239's tests, SQLx and
server-Clippy derivations completed, not its whole flake.

Completed bounded identity review with ponytail and Rust skills: minimal session
and directory fields, same-org approval-name join without excluding archived
submitters, preserved totals/filters/actions, no policy activation. New Project's
financial DTO and Harvest compatibility API stay separate. Current-master Clients
and invoice recovery retain their consumed fields and navigation coverage.

Correction to earlier inventory: the final directory HTTP file also includes
`1eb13ec`'s canonical `check_scoped`. Extracted the original `7f14e4b` legacy
matrix with exactly `3bb62ac`'s four superseded own-user assertions removed;
the preserved session matrix replaces them. The canonical helper/call stays owned
by `1eb13ec` in the original backup, not deleted or waived.

Created independent branch/worktree `fix/identity-response-projections` /
`.worktrees/identity-response-projections`, unsigned `6ff00f0`: 14 files,
689 insertions/88 deletions. Six complete model/page/test blobs match both
`3bb62ac` and final `db3935d`. Remaining production changes are selected original
hunks; harness changes only register the three relevant matrices. Navigation
fixture changes preserve master's member branch and expanded Clients tests.
No new schema, CSS, dependency or canonical endpoint. Format `25858` passed
unchanged. Suite followed by SQLx regeneration runs in `19675` on disposable
PostgreSQL. The initial touch used a nonexistent core path; no file was created,
and compiler output confirms actual `crates/core` and app are rebuilt here.

Ran Spec Kit analyze's prerequisite command once in the original worktree;
feature 015 resolves and no extension hooks exist. Applicable spec/plan/tasks,
contracts and constitution 1.1.0 were read. Nine tasks T145–T147/T151–T153/
T162–T164 map to FR-002/006/008/010/018 and the relevant SC-006 regressions.
Six requirement IDs have bounded task coverage; no unmapped tasks or new
ambiguity, duplication, critical/high or constitutional finding. One low note:
historical T150→T151 sequencing is not a runtime dependency on the canonical
directory. No feature artifact changed and no full-feature acceptance is claimed.

Next: collect `19675`, verify cache provenance, run offline server/WASM lints,
publish the draft and start clean-head full Nix. Do not use the shared local
Cargo target until this suite/cache process finishes. Mixed specification hunks
and remaining groups still need ownership; original worktrees and #208 untouched,
no merges, original closures or real data changes.

Identity suite/cache `19675` exited zero: 818 app, 183 integration and 121 core
tests, 1,122 passed with 11 existing manual tests ignored. The production-page
approval tests, real-session matrices and 44 detail-navigation tests passed.
Regenerated cache has 1,001 unchanged base descriptors, eight byte-identical
original additions and two obsolete removals, 1,009 total. Seven additions
originated in the two selected query commits; one fixture descriptor already
existed through `3ae8e08` in the original parent, with no related implementation
imported. Unsigned cache commit `1ce993f75092ca5dc81eb5f96d2a51a6537a8d6d`.

Published independent draft [#240](https://github.com/numtide/horae/pull/240).
Remote inspection confirms master base, exact head, 23 files and 871 insertions/
185 deletions including generated metadata (Git recognizes one cache rename).
Offline server then WASM lint runs sequentially in `38653`; full clean-head
Nix runs separately in `16586`. Both are live, not counted as passed. The shared
local Cargo target remains occupied by `38653`.

Read-only next-boundary inventory: `5ec183a` fences six interactive time
transaction entry points against current account deactivation; original
`activity_tests.rs` is unchanged through final `db3935d`. Its dependencies are
the organization lock helper and bounded READ COMMITTED configuration, not the
complete canonical permission module. Read its contract, source diff and all
four activity tests. Later `60f60f9`, `5faed76`, `48a6533` and `02c4245`
modify the shared time file and stay separately accounted. Complete caller,
service-barrier and current-base review before choosing its smallest valid base;
no time extraction has been created. `c80233b` separately repairs time-entry
invoice-ID serialization, with legacy HTTP checks embedded in the canonical
time matrix: retain those checks if extracting it independently, not the unrelated
canonical setup. Neither inventory item is runtime verification or feature work.

### 2026-10-06 — Time-writer activity extraction

The intervening goal-prompt response made no separation progress. Revalidated
the next safe action: #216 is merged at `02f7b58`, original tracked backup and
untracked archive comparisons pass, and the next boundary has not been extracted.
No original branch, worktree or #208 was changed.

#238 full Nix `24069` exited zero on `7a7cede`: all local x86_64-linux checks,
including browser and both NixOS VM suites, pass. Its PR description now records
this result; kept draft for coordinated review. #239 `87204` and #240 `16586`
were confirmed live. #240 offline server/WASM lint `38653` passed and PR-body
update `1841` completed; the shared local Cargo target became available.

Completed the time-writer caller/service/approval review. The existing submission
path takes the exclusive user advisory barrier before entries and only compatible
organization/user FK locks; the new interactive prefix never upgrades its
organization SHARE lock. Legacy user deactivation and project access ordering
are provided by #227/#228. Imports still use the unchanged lower-level advisory
helper; no service execution is recast as a user operation.

Created `fix/time-write-activity` in `.worktrees/time-write-activity` on #228
`0e1e6759cff0b80a046ced4694a78022f096ab3d`. Unsigned source commit `3ea235f`
extracts `5ec183a` T171–T173: four files, 402 insertions and 22 deletions.
`db.rs` and `update_tests.rs` match the original commit. The 300-line activity
test file matches both `5ec183a` and final `db3935d`, blob
`2935efd06339ea75164a7708da637652c5c3ce81`. Every original assertion is retained.
The time module differs only by preserving the base's absence of the unrelated
canonical reader and inlining the original two transaction-configuration queries
from `permissions::configure_administration`. SQL strings and behavior are
unchanged; this avoids importing the complete canonical permissions module for
one call. No schema, CSS, API surface, dependency or policy activation is added.

Read and applied ponytail, Rust best-practice, testing and async guidance. Ran
Spec Kit analyze's prerequisite command in the original worktree (015 resolves;
no extension hooks), read the relevant spec/plan/tasks/contract and constitution.
Bounded coverage: FR-010 and SC-003 map to T171/T172; FR-018 and SC-006 map to
T171/T173. Four applicable requirement/criterion IDs, three tasks, all mapped;
no ambiguity, duplication, constitutional conflict or critical/high finding in
this boundary. This is not full-feature acceptance. Original specification
hunks remain preserved for the documentation reconciliation, not silently lost.

Formatting `46067` passed with zero changes; staged diff checks pass. Disposable
PostgreSQL workspace suite followed by SQLx regeneration is live in `64428`.
It owns the shared local Cargo target; do not start another local Cargo process.
Next: inspect its result and regenerated cache provenance, then run offline
server/WASM lint, publish the bounded draft stacked on #228 and start clean-head
Nix. Original full cross-command/delegated-policy gates remain separate work.

#239 full Nix `87204` subsequently exited zero on unchanged `66a256f`:
all local x86_64-linux checks passed, including browser and both VM suites.
The time extraction compiled successfully in `64428` and advanced to tests;
the combined suite/cache process is not yet complete. #240 `16586` remains live.

Next-boundary read-only inventory: `c80233b`'s complete time-entry model change
is independent of canonical grants and UI. Its original real-session legacy
matrix precedes policy setup inside `scoped_time.rs`; preserve the fixture and
all assertions when registering it independently in the existing singleton
HTTP harness. Current-master time UI uses state, not `invoice_id`; invoice
recovery and the Harvest compatibility API use distinct models. No payload
extraction has been created yet and this consumer inventory is not runtime proof.

Time suite/cache `64428` exited zero: 841 app, 180 integration and 121 core tests
(1,142 passed; 11 existing manual tests ignored). All five extracted regressions
and existing submission, task-revocation, user-access and invoice tests pass on
source `3ea235fa271d9caf3de9674cba949b9c7fb4e118`. Full SQLx regeneration
retains all 1,031 base descriptors unchanged and adds 11, each byte-identical to
`5ec183a`; six were already present in its original parent. No removals or
modified base descriptors. Unsigned cache commit
`7820f8da64ef52169c23e2967a3c16a1a74070a8`.

Published draft [#241](https://github.com/numtide/horae/pull/241), stacked on
`fix/project-access-lock-order` (#228). Its 15-file diff contains four source/test
files and 11 generated descriptors, 614 insertions and 22 deletions. Required
integration order is #227 → #228 → #241; retarget and revalidate after prerequisites
land. Combined offline server/WASM lint `74243` and clean-head full Nix `85167`
are live on `7820f8d`, not counted as passed. The shared local Cargo target is
owned by `74243`. #240 full Nix `16586` remains live, with its test derivation
passed but no whole-flake completion yet.

This iteration is progress: original time-writer work is now isolated, published,
source-accounted and suite/cache-verified; #238/#239 full local Nix results are
recorded in their PRs. Next collect the live gates and continue the separately
inventoried time-entry payload boundary without importing canonical reader setup.
Remaining original feature/UI/specification/governance hunks and unfinished local
Clients work still prevent completion of the overall separation goal.

### 2026-10-06 — Time-entry response boundary

Previous iteration classified as progress: #241 was published with suite/cache
evidence and the preservation ledger pushed. Re-read the objective, confirmed
#216 merged, fetched unchanged master `1b8fa4f` and revalidated all three live
handles. Original tracked snapshot comparison still passes. #241 offline lint
`74243` subsequently exited zero: server and WASM passed on `7820f8d`. Updated
its PR body and the delivery/accounting tables, including #238/#239 full local
Nix completion. #241 full Nix `85167` and #240 `16586` remain live.

Created independent `fix/time-entry-payload` in `.worktrees/time-entry-payload`
on master `1b8fa4f`. Unsigned source `3ff9b8818716473c1ec2a674480cd634ad532ae3`
contains three source/test files, 128 additions, extracting `c80233b` T168–T170.
The complete shared model matches both that commit and final `db3935d`, blob
`cbfba91bba08e68f38be22b33cc643c423e177a8`. No later original commit modifies it.

Preserved the original populated-invoice HTTP fixture and complete legacy-role
loop in `authorization_tests/time_entry_payload.rs`, registered in the existing
singleton HTTP matrix. Only the unrelated canonical import, unused actor-cookie
lookup and subsequent canonical setup/checks were excluded. Those checks remain
owned by the canonical time reader, not discarded. Session-only identity, foreign
input, exact field omission, one returned owned entry and the unchanged stored
invoice relationship are still asserted for all three legacy roles.

Applied ponytail and Rust/test guidance. Bounded review traced shared response
sites, SQLx decoding, model input/output and all invoice-id consumer references:
the time UI uses state; plugin payloads and invoice/report/Harvest projections
use separate models. No production query, schema, CSS, dependency, action grant
or canonical activation changed. Existing contract and task provenance remains
FR-008/018; this is not a fresh full-feature Spec Kit acceptance claim.

Formatting `88301` passed unchanged; diff checks and model provenance pass.
Workspace suite plus complete SQLx regeneration runs on disposable PostgreSQL
in `37602`, using the shared local Cargo target now released by #241's lints.
Do not start another local Cargo command until it finishes. Next verify cache
provenance, run both offline lints, publish the independent draft and start Nix.

Read-only subsequent-boundary inventory: project-manager delegation was introduced
by `9a7e05d`, hardened by `202ee96`, exposed by `84d5352` and made composable
with project editing by `2497dbe`. Final models/readers/commands and transaction
tests must be reviewed together; `PermissionRequester` is a small shared DTO
absent from the current #234 model. Actual organization-lock and schema dependencies
must be resolved before extraction. No delegation code was changed or certified.

Time-payload suite/cache `37602` exited zero: 821 app, 180 integration and 121
core tests (1,122 passed; 11 existing manual tests ignored). Complete SQLx
regeneration retains all 1,003 base descriptors unchanged and adds three,
byte-identical to original `c80233b`; 1,006 total, no removals or modified base
descriptors. Verified the extracted HTTP fixture/legacy loop equals the original
slice after only the documented import/cookie/comment adaptations. Unsigned cache
commit `43337fc4180b8a0c590408f5525a2bddf4c7567f`.

Published independent draft [#242](https://github.com/numtide/horae/pull/242) on
master, six files and 173 additions (three Rust/test files and three generated
descriptors). Remote inspection confirms the exact head, base, draft flag and
diff size. No critical/high finding in the bounded payload review; full gates
are still pending. Offline server/WASM lint `22698` and clean-head Nix `73808`
are live on `43337fc`. The shared local Cargo target is occupied by `22698`.
#240 `16586` has completed release compilation and advanced to browser/VM checks;
#241 `85167` has passed its SQLx and server-Clippy derivations but remains live.
Neither whole-flake result is inferred from partial success.

Further delegation dependency evidence: the shared requester DTO originated in
`6bba224`; preserve its exact eight-line definition without importing the entire
editor. #234 has the receipt/assignment schema and profile/template foundations
but lacks `db::lock_organization`, so it alone is not a valid base for the final
composable command. Read the final session wrappers and strict DTOs; subsequent
whole-file command, transaction/reader/HTTP test and contract review remains
required before creating that extraction. No new decision or feature is needed.

This iteration is progress: #242 is isolated, source-accounted, suite/cache-verified
and published. Next collect the four live checks without restarting them, then
resolve and review the project-delegation boundary. Remaining canonical/UI/spec
and unpublished Clients groups still require final ownership; goal not complete.

### 2026-10-06 — Composable project delegation extraction

The immediately preceding turn only supplied the requested goal text: no-progress
for the existing separation goal. This iteration re-read that goal, reconfirmed
#216 merged at `02f7b58`, and checked the three existing Nix handles live without
restarting them. Original tracked snapshot and untracked archive comparisons both
pass. #242 offline server/WASM lint `22698` completed successfully on `43337fc`;
its PR body is updated. #240 `16586`, #241 `85167` and #242 `73808` remain live.

Read the complete final delegation command, strict DTOs, session wrappers, all
three PostgreSQL test files, HTTP fixture, shared transaction helper and the
project-management command contract. The final command needs both #234's schema/
profile-template foundations and #228's shared organization-lock helper. Created
isolated review base `integration/project-delegation-prerequisites` at
`e44433e3dea8ab92b26149f2b98a2af122086092`, combining exact #234 `45d219e` and
#228 `0e1e675` without conflicts. This local integration base is not a delivery
PR or a GitHub merge target. No existing branch was moved.

Created `feat/project-manager-delegation` in its own worktree on that base.
Unsigned source commit `be787cad1405ff42a3f93c0ef2604fa5bc3587eb` has 12 files,
2,334 additions and one deletion. This is one coherent command/API boundary,
mostly existing tests, not a new permissions feature. Original provenance:
`9a7e05d`, `202ee96`, `84d5352`, composability hunks from `2497dbe`, and the
eight-line requester DTO from `6bba224`.

Six full files match final `db3935d` byte-for-byte: project-manager DTOs, session
wrappers, command, command tests, read tests and transaction tests. Root module
registrations are minimal. The requester DTO is shared on server/WASM; existing
profile/template command types retain their server-only compilation boundary
using item-level cfg attributes. No schema, dependency, UI, style or activation
change was added. Legacy project saves remain unchanged.

The original HTTP test is preserved except for its audit-endpoint block starting
at `let request_id: Uuid` and ending immediately before the subsequent
`grants(...ProjectWriteManaged...)` call. Its two forbidden assertions for
`get_permission_audit` and `list_permission_audit` require the separately
retained audit API and explicitly belong to that extraction/integration suite.
They are not waived or reported passed here. The original file and its fixture
remain recoverable unchanged. All delegation route, requester mismatch, payload,
replay, stale revision, retained inactive target, self-removal, malformed-state,
same-cookie revocation and receipt-count assertions are otherwise exact.

Bounded adversarial review traced session identity through current policy/grant/
designation checks, organization-first serialization, actor/added-target SHARE,
project KEY SHARE NOWAIT, exact normalized intent, durable receipt/audit,
overflow, no-op preservation and whole-transaction rollback. Current authority
is checked before replay; retained targets are not silently removed for lost
eligibility. Membership, rates, costs, history and other management relationships
remain unchanged. Composition does not commit or change caller isolation.
No critical/high finding in this extracted boundary; runtime tests remain gates.

Ran Spec Kit analyze's prerequisite command once with
`SPECIFY_FEATURE=015-scoped-permissions`, from the original worktree; required
artifacts exist and no extension hooks are configured. Read its constitution,
relevant specification/plan/task sections and full delegation contract, without
editing original specs. Scoped report:

| Requirements | Tasks | Coverage / limitation |
| --- | --- | --- |
| FR-005/011/026 | T059–T061, T189–T191 | Distinct scope, current project-edit authority, compatible additions; no grant promotion |
| FR-010 | T133–T135, T189–T191 | Both activity race orders, revocation, cancellation and current replay authority |
| FR-013 | T059–T061, T189–T191 | Atomic receipt/audit and sanitized payload; audit-reader assertions owned separately |
| FR-017 | T059–T061 | Membership, money and history preserved; composition commit/rollback tested |

Six scoped FRs, nine mapped tasks, 100% task coverage, zero unmapped tasks and
zero critical/high specification findings in this boundary. SC-002/003/006 have
corresponding payload/concurrency/regression tests but full-feature success is
not claimed. Master constitution 1.0 and the original proposed 1.1 governance
reconciliation remain separately inventoried; this extraction adopts no amendment.
Next analysis action is to validate the extracted source, then restore audit
integration assertions with their owning API; no speculative spec edits needed.

Formatting `82393` passed unchanged and diff checks pass. Workspace suite plus
complete SQLx regeneration `4502` is running on disposable PostgreSQL, occupying
the shared local Cargo target. Next collect it, verify every generated descriptor,
run offline server/WASM lint and full Nix, publish the bounded draft and update
this record. Overall separation remains incomplete.

Delegation suite/cache `4502` exited zero: 926 app + 180 integration + 189 core =
1,295 passing tests; 11 existing manual tests ignored. Regeneration preserves all
1,125 base SQLx descriptors unchanged and adds 46 byte-identical to final original
`db3935d`, 1,171 total; no deletion, modification or unmatched addition.
Verified the HTTP file equals the original after removing exactly the recorded
audit block. Unsigned cache commit `1818bbf38453e13ea59e49fde16832ac84c4b2ab`.

Published draft [#243](https://github.com/numtide/horae/pull/243); remote verification
confirms the exact head/base, draft flag and 58-file diff (12 Rust source/test files
plus 46 SQLx descriptors, 3,197 additions and one deletion). Both branches are
pushed. Offline lint `56569` and full Nix `22335` run on the cache-inclusive head;
Nix's formatting derivation has passed but the whole run is not yet complete.
The shared local Cargo target is occupied by `56569`.

Read-only next-boundary inventory: audit history comes from `fc85231`,
`300d1e9` and `8722320`. Read the complete final audit backend, historical DTOs,
two session wrappers and audit-lookup contract; inspected test/UI dependency
registrations, not the full test/UI implementation. Audit fixtures use actual
template/profile/delegation commands, so #243 is a direct prerequisite. The
browsable page also depends on shared permission descriptions, own-access
navigation and canonical shell admission; resolve that coherent delivery boundary
before extraction rather than dropping those implemented UI paths. Restore the
two deferred delegation/audit HTTP assertions with the audit API. No audit
extraction or UI certification is claimed yet.

This iteration is PROGRESS: #243 is published, original-code/accounting evidence
and 1,295 tests plus complete cache validation are recorded. Next collect its
offline lint and existing Nix handles, then review/extract the audit-history
boundary. #240 `16586`, #241 `85167`, #242 `73808` remain live; #241 advanced
through release build into browser/VM checks, not a whole-flake pass. No merges,
original closures, policy activation, data changes or new features.

### Own-permission extraction and delegation web lint boundary

The preceding prompt-only response made no implementation progress. Revalidated
the next safe action: #216 is merged at `02f7b58`; the original tracked snapshot
still matches its backup. Polled the actual verification handles, not CI on a
timer. #241 full Nix `85167` exited zero on `7820f8d`, including browser and VM
checks; its PR body now records the result. #240 `16586` and #242 `73808` remain
live, so no complete result is inferred from silence or individual derivations.

#243 initial offline lint `56569` passed server but failed WASM on unused DTOs.
The separate UI consumer is not in this extraction. The first conditional
expectation change (`9c6b121`, twelve lines) was too broad: `40252` passed server
but reported two unfulfilled nested expectations in WASM. Corrected to a single
`expect(dead_code)` on the unused `ProjectManagers` response root, following the
existing command/outcome convention. Final unsigned `3404c85` differs from
`1818bbf` by only four conditional-attribute lines; DTO fields, wire types, SQL,
runtime code and tests are unchanged. Offline WASM `49707` exited zero on this
source. Five complete command/wrapper/database-test files still match the
original; the DTO file now has this explicitly accounted adaptation. Current-head
full Nix `87574` is running. Older Nix `22335` (`1818bbf`) and `90130` (`9c6b121`)
were explicitly interrupted after confirming their PID, worktree and start time
(`3340917` and `3413583`); both handles exited one with the expected interruption.
They are not verification of the final head. The final-head check is retained.
Final-head server
verification remains pending, despite the unchanged server compilation path.

Audit-history dependency review identified the existing own-access Settings
consumer as a coherent prerequisite. Created `feat/own-permission-settings` and
its isolated worktree on #234 `45d219e`, preserving the original reader, display
DTO, seven database tests, registered-cookie checks, Settings section, exhaustive
grant descriptions, nine SSR tests and production-resource refresh test. Unsigned
source commit `0b0da87` contains 14 files and 1,115 added lines; no migration,
dependency, CSS, legacy guard replacement or policy activation.

Six complete files match final `db3935d` byte-for-byte; the component and shared
description file match `8c15bfe`. Their later audit link and `profile_label` helper
remain owned by the audit extraction, not discarded. `get_my_permissions` retains
the final `8722320` authentication-error sanitization. Existing `get_me` stays
unchanged here because its separate projection is #240. General and Plugins are
unchanged apart from insertion of the independently loading section.

Scoped Spec Kit analyze ran the prerequisite script against original feature 015
with no extension hooks. FR-006/007/010/012/016/017/018 map to T095–T097 and
T120–T122: seven requirements, six tasks, all mapped, no critical/high,
duplication or ambiguity finding in this boundary. SC-002/003/005/006 remain
partial; T018 browser/Workspace acceptance and the original proposed governance
amendment are not declared complete or silently adopted.

Source review traced session-only identity, explicit policy/active-actor checks,
organization-before-actor locking, strict canonical restore, sorted own scopes,
non-mutating reads and sanitized errors. UI review verified exact selected grants,
independent administrator identity, no raw IDs/revisions, loading suppression,
legacy/empty/error distinctions and guarded refresh. No critical/high source
finding in this boundary. `nix fmt` `92652` passed with zero changes; Impeccable's
manual detector returned `[]`. These are not contrast, keyboard or viewport
certification. No rendered browser acceptance is claimed.

Disposable full workspace suite and SQLx generation run in `50483`; the shared
local Cargo target is now occupied by that process. Next collect its result,
verify cache provenance, run offline lints and full Nix, then publish a scoped
draft with explicit browser limitations. Continue with audit history afterwards,
restoring #243's deferred audit-denial assertions. This iteration is PROGRESS;
the overall separation is still incomplete. No merges or original closures.

### Own-permission delivery published; isolated browser verification added

Previous iteration is PROGRESS: the original own-permission flow was extracted
and the delegation lint boundary corrected. This iteration revalidated #216 as
merged and polled the existing live handles without restarting them. #242 full
Nix `73808` exited zero on `43337fc`, including browser and VM gates; its PR body
now records the confirmed result. #240 `16586` remains live and quiet. A read-only
diagnostic retrieved an empty current browser-derivation log; that supplies no
failure diagnosis or reason to restart the live check.

Own-permission suite/cache `50483` exited zero: 893 app + 191 integration + 189
core = 1,273 tests; 11 existing manual tests ignored. Source under test is
`0b0da87`; browser-only follow-up `12cac34` does not alter that Rust source.
Complete regeneration retains all 1,098 base SQLx descriptors byte-for-byte and
adds 26 identical to final original `db3935d`, 1,124 total, no removals or modified
descriptors. Unsigned cache commit `6c4e4d134b9c2924085e56d504c3a684752fb7c4`.

Inspection of the original browser suite found its Settings visit only exercises
the later audit link, not this independent own-permission consumer. Added the
bounded `own-permissions.cjs` verification and registered it in the existing
disposable runner (`12cac34`, 113 additions/one deletion, no production edits).
The test validates the private test socket/URL before fixture writes, exercises
real session reads, legacy-to-canonical explanation, exact grants, independent
administrator identity, native keyboard refresh, pending-content suppression,
invalid-state recovery and deactivation. Geometry checks cover both themes,
320/390/768/1440 widths, a short viewport and CSS zoom 2. This last case is not
native text zoom. Optional two-theme screenshots are available for inspection.
Fixtures are restored before the runner stops its temporary PostgreSQL.

Node syntax `7136` and formatting `32758` passed; no rendered-browser pass is
claimed yet. This is verification of preserved functionality, not a new feature.
The original files, legacy controls and CSS remain unchanged. The broader T018
Workspace/native text-zoom/full visual acceptance is still separate.

Published draft [#244](https://github.com/numtide/horae/pull/244), stacked on #234's
`refactor/person-profile-commands` (`45d219e`), head `6c4e4d1`: 42 files (16
source/test/registration files plus 26 SQLx), 1,674 additions and one deletion.
Offline server/WASM lint `53333` and full clean-head Nix `19313` are running on
that exact head. The full gate includes the new isolated browser test. The shared
local Cargo target is occupied by `53333`; do not overlap another local build.

Next collect those results and inspect the optional Settings captures using the
built package once available. Continue the coherent audit-history extraction on
#243 + #244: source inventory confirms its backend/UI derive from `fc85231`,
`300d1e9` and `8722320`. The `8722320` admin-shell revision gates only Audit with
own canonical identity and preserves the incumbent People/Importers guards;
later People/Tasks shell changes must retain their separate ownership. Restore
the deferred Settings audit link, profile labels and #243 audit-denial assertions
with those endpoints. No audit implementation is claimed in this iteration.

This iteration is PROGRESS: #244 is published with full Rust/cache evidence and
explicit browser/lint/Nix limitations; #242's full gate is now confirmed. #243
current-head Nix `87574` and #240 `16586` remain live. Original tracked backup
comparison still passes. No merges, original closures, policy activation or real
data changes; the overall separation remains incomplete.

### Permission history extracted; own-permission browser evidence confirmed

The preceding user-facing turn only supplied a goal prompt: NO PROGRESS toward
the existing separation objective. This iteration reread that objective,
revalidated #216 as merged, and made concrete progress without changing scope.

#244 offline server/WASM lint `53333` exited zero on `6c4e4d1`. The exact Nix
package subsequently became available at
`/nix/store/2kv7laqd6z31ibgrc5c0hfnq5ckvaimx-horae-0.1.0`.
Focused disposable browser run `47268` exited zero in Chromium 148.0.7778.96:
real session reads, legacy/canonical distinction, exact grants, explicit admin
identity, keyboard refresh, pending suppression, malformed-state recovery,
deactivation and both-theme responsive/CSS-zoom cases passed. Desktop-dark and
390px-light full-page captures were inspected in
`.worktrees/own-permission-settings/.scratch/browser-own-permissions-6c4e4d1/`;
no section clipping or overlap was observed. The test deliberately sets the DOM
theme, so the unchanged General selector can still say Dark in the light capture.
No native text-zoom, touch gesture, cross-browser or contrast certification is
claimed. The PR body now records the actual evidence; a transient GitHub GraphQL
failure was retried successfully.

Created and published review base
`integration/permission-audit-prerequisites` at
`59d27981db88bfa2c26bd96f39bff995b45901e3`, combining exact #243 `3404c85` and
#244 `6c4e4d1`. Two module/test-registration conflicts were resolved by retaining
both modules and both HTTP checks. This is a local composition, not a GitHub PR
merge or a future merge target. After both prerequisite chains reach master,
retarget/rebase and verify the delivery there.

Created and published `feat/permission-audit-history` in its own worktree.
Unsigned source head `c96d78765162a9a9a695e88840c2bcd34677cd0c` has 18 files,
3,211 additions and 13 deletions. It extracts the complete historical DTO,
single-record and paged readers, strict decoder, database/concurrency tests,
registered-cookie tests, native history details, requester-bound pagination,
Settings navigation and canonical Audit-only shell gate. Existing People and
Importers guards remain unchanged; no later People/Tasks shell changes, editor,
migration, CSS, dependencies or activation are included.

Ten complete files match final original `db3935d` byte-for-byte. Admin shell and
its tests match `8722320` after only adapting the two links and mocked route to
the base's existing Timesheet signature (no delegated-user selector). Restored
the complete 14-line audit-denial block deferred from #243's registered-session
test, plus #244's deferred Settings link and shared profile labels. The original
browser recovery suite's audit section remains preserved; it relies on the
not-yet-extracted editor. Reconcile that browser ownership and independently
exercise the audit consumer before declaring the delivery verified.

Scoped Spec Kit analyze ran original feature 015 prerequisites with no extension
hooks. Coverage is five requirements / ten tasks: FR-010/011/013 map to
T062–T064, T123–T125 and T182–T184; FR-016 maps to T183/T185; FR-018 maps to
T064/T125/T184/T185. All five have task coverage, no unmapped scoped task, no
critical/high ambiguity, duplication or constitutional conflict in this boundary.
T185 delivery evidence is pending here, not inherited from original #212.
Full T018/T041/T042, the broader SC-002/003/005/006 outcomes and proposed
governance amendment are not closed or adopted.

Bounded source review verified authorization before receipt lookup, tenant-only
25+1 paging, fresh READ COMMITTED with local limits, organization-before-actor
locks, strict historical decoding without raw intent/replay disclosure, explicit
admin identity, revocation/cancellation coverage and non-mutating projection.
UI review verified original escaped detail rendering, native disclosure controls,
requester binding, stale-content suppression and safe error/retry states.
No critical/high finding in that source boundary. Formatting `72211` passed with
zero changes; one detector invocation returned `[]`. These do not certify
rendered history or all accessibility dimensions.

Full Rust workspace execution in `96253` passed on `c96d787`: 969 app +
198 integration + 189 core = 1,356 tests; 11 existing manual tests ignored.
Its subsequent SQLx preparation did NOT pass: the combined process exited 143
after the private socket disappeared. The temporary PostgreSQL log
`/tmp/horae-import-cleanup-pg.94GG9k/log` records a fast shutdown at 19:49:11 UTC;
the PID file is gone and no local Cargo process remains. The source of the
termination is not established. Do not treat partially regenerated caches as valid.
Cache-only retry `62209` uses a fresh disposable database; the successful tests
are not rerun. Shared local Cargo target is occupied by that retry.

Next collect `62209`, prove complete SQLx provenance, commit the cache, run
offline native/WASM lint and full Nix on the final head, then publish a scoped
draft with its browser limitations. Continue the independent browser history
verification and remaining original ownership inventory. #244 Nix `19313`,
#243 `87574` and #240 `16586` were confirmed live this iteration; no full-pass
claim yet. Original tracked work still exactly matches its saved snapshot.

This iteration is PROGRESS. No merges, original closures, real-data writes or
policy activation occurred. The overall goal remains incomplete.

### Audit cache recovered and draft published

Cache-only `62209` exited zero but provenance correctly rejected its output:
one existing CLI-restart query was missing. Touching only that test and preparing
again (`6814`, exit zero) produced only three descriptors, proving that a
successful command can reuse other targets without regenerating their caches.
Neither incomplete result was committed. Invalidated timestamps for all Rust
source files in this isolated worktree and ran the same complete preparation
again (`69078`, exit zero). No source content changed and no successful test
suite was repeated.

Final provenance passed: all 1,195 base descriptors unchanged, 30 additions
byte-identical to original `db3935d`, 1,225 total, zero removals/modifications or
unmatched additions. Unsigned cache head
`533922ad904b4d93abed4a1db917086bb15feeed` is published in draft
[#245](https://github.com/numtide/horae/pull/245), on review base `59d2798`.
Offline server/WASM lint `72147` and full clean-head Nix `61137` are running.
The Nix formatting gate has passed; no full Nix result is claimed.

Next collect these exact handles, preserve the original editor-dependent history
browser assertions, and verify this historical consumer independently with real
writer-produced receipts in disposable fixtures. #243 already exposes the
session-bound project-manager command, so real history can be generated without
extracting the unfinished editor merely to obtain browser fixtures. All existing
template/profile historical tests and the original browser checks remain owned,
not replaced or dropped. Continue remaining original ownership after this
delivery's verification. No merges or activation.

### Final foundation checks confirmed; independent history browser test published

Re-read the active separation objective and confirmed #216 is merged at
`02f7b58acdcf126415f9ec89215da8cdada7d03f`. The preceding conversational turn
only supplied a proposed goal prompt and made no repository progress. This
iteration resumes the actual attached objective, not that proposed replacement.

Old terminal handles for #243/#244 were already closed. Re-evaluated their
unchanged clean final heads through `nix flake check -L`: `10972` (#243,
`3404c85`) and `30710` (#244, `6c4e4d1`) exited zero, all x86_64-linux outputs
already cached, zero checks rebuilt. This confirms final native/browser/SQLx/VM
gates and closes #243's last native-lint uncertainty. No cross-platform build
claim is made. Their published PR descriptions now record these results.

#245 offline server/WASM lint `72147` exited zero on `533922a`. Nix `61137`
remains live; SQLx, Clippy and Rust test build phases completed, but the package
was still unavailable at the focused browser launch attempt. That attempt
stopped at `test -x`, before creating any database or launching a browser.
Do not restart this build on observation timeout or count it as the complete
gate for the subsequent test-only head.

Published `9744184` and `065a96b` on the existing draft #245. They add only
`permission-history.cjs` and its registration, retaining every previous runner
suite. The disposable fixture uses the exact compiled Dioxus endpoint string
(the pinned macro hashes implicit routes), current-session HTTP commands,
complete project grant prerequisites and no fabricated receipt JSON. Its 27
changes plus one no-op drive empty state, exact 25/3 paging, native keyboard
disclosure, narrow/light and desktop/dark layout, stale-history suppression,
revocation/recovery, canonical-admin navigation despite legacy Member role and
deactivation. Fixture cleanup restores only the isolated seeded organization.
The original editor-dependent assertions, including deleted custom profiles,
remain owned by the later editor integration; they are not waived or replaced.

Syntax checks `23309` and `75082` passed; formatter `97076` passed with zero
changes before the final fixture-only prerequisite correction. Runtime and
final-head gates remain pending, explicitly reflected in the PR body. No Rust,
schema, SQLx descriptor, CSS or product behavior changed. Source preservation
check against `backup/pr212-split-20261006-uncommitted` still passed.

#240 Nix `16586` was confirmed live. Read-only process inspection identified
its active Node child `3237506` running the immutable store copy of
`action-errors.cjs` for more than 85 minutes, with Chromium alive. Its Nix log
was empty. That suite has event/response waits without explicit deadlines;
the particular blocked wait and root cause are not yet established. No process
was killed or restarted. Next diagnose with a bounded isolated reproduction
if needed, without treating a stall as either a pass or a product regression.

Next collect `61137`; once its exact package is available, run the focused
history suite against that unchanged production code, inspect both captures,
resolve any proven failure, and verify the final test-inclusive head. Continue
remaining canonical consumer/editor and specification ownership afterward.
This iteration is PROGRESS. No merges, original closures, real-data changes or
policy activation occurred; the overall goal remains incomplete.

### History browser verified; bounded action tests and preflight extraction

The previous goal iteration was PROGRESS (published browser verification and
updated authoritative check evidence). Re-read the attached objective; no scope
or completion criteria changed.

#245's built `533922a` package became available. Focused browser `56883`
failed with `Route is already handled!`: the test released a held request and
removed its interceptor before the handler finished. Preserved this failure;
the application had not been shown faulty. Test-only `93f230d` moves interceptor
removal after the released response renders. Complete focused run `36324`
then exited zero in Chromium 148.0.7778.96. All real-writer, empty/no-op, exact
25/3 paging, keyboard, pending suppression, revoke/recover, canonical navigation
and deactivation assertions passed. Inspected the desktop-dark and narrow-light
captures in `.scratch/browser-history-coordinated/`: no page overflow or overlap
observed. No native text-zoom, touch, contrast or cross-browser certification.
Format `79897` passed without changes. Final-head full Nix `9966` is running
on `93f230d`; old `61137` on `533922a` continues but cannot substitute for it.

#240's original Nix remained live in `action-errors.cjs`. An isolated diagnostic
(`4807`) kept every assertion but set finite waits and logged request paths;
it passed, so the original blocked wait/root cause is still unproven. Committed
and published only `page.setDefaultTimeout(30_000)` as `d2da193`. This bounds
previously infinite event waits rather than increasing assertion timeouts.
Exact uninstrumented focused run `47505` passed all scenarios on the unchanged
built application; format `50726` passed. Explicitly interrupted the superseded
Nix PID `3032398`; handle `16586` exited 1 with interruption, not a green result.
Final-head full Nix `79480` is running. PR descriptions retain the inconclusive
history, bounded reproduction and pending final gates.

Scoped Spec Kit analyze ran feature-015 prerequisites successfully with no
extension hooks. Preflight coverage is five requirements (FR-006/010/014/017/018)
and three mapped tasks (T117–T119), 100% scoped requirement coverage, no unmapped
tasks, ambiguity, duplication or critical/high conflict in this diagnostic
boundary. US5/SC-004 and T019 migration/activation remain incomplete; the proposed
constitution amendment is not adopted. No specification was regenerated.

Created isolated `feat/permission-preflight`. Source reader (91 lines) and full
tests (451 lines, seven cases) match original `db3935d` blobs exactly:
`ed8377fbd45dae76791c766ac14574b3f298220c` and
`73d354d1bbbbfdae86ecb823ee329736581dee0b`. The only other change is module/test
registration. Review covers current legacy Administrator before counts, policy
zero, organization-then-actor locks, post-wait checks, one-statement snapshot,
count-only tenant diagnostics, cancellation/lock failure and before/after full
stored-value comparisons. Zero counts confer no activation readiness. No public
endpoint, new schema, CLI, UI, product behavior or real-data inspection is added.

Initial source `a88e849` used #237 alone. Compile-time SQLx in `26167` caught a
missing prerequisite: the preservation test also snapshots
`permission_change_receipts`, whose migration is in #226. Did not delete or
conditionalize that assertion. Explicitly stopped the already-failed Cargo PID
`3811415`; wrapper exited 101 after its interrupted child. No passing suite or
cache was claimed from that attempt.

Published review-only base `integration/permission-preflight-prerequisites`
at `61c90bc`, combining exact #237 `2242361` and #226 `82d15f3`. Two composition
conflicts retained #226's strict-storage superset and server-only template DTO
registration; no #237 behavior was discarded. Rebased only the new extraction
onto that base as `b159184`, preserving both preflight blobs and both test/module
registrations, 3 files / 547 added lines. Updated only that new remote branch
using an exact lease against `a88e849`; original branches/backups remain intact.
The integration branch is not a delivery merge target. Both prerequisite chains
must reach master before retargeting and repeating final-head gates there.

Corrected full disposable suite and complete SQLx preparation are running in
`28480`; the reusable local Cargo target is occupied. All worktree Rust-source
timestamps were invalidated before preparation to avoid stale cached-target
omissions. Original unpublished tracked work still exactly matches its saved
snapshot. Next collect `28480`, prove complete cache provenance, run offline
native/WASM and full final-head Nix, and publish the scoped draft. Also collect
`9966`/#245 and `79480`/#240 without restarting live handles. Continue canonical
consumer/editor and specification ownership after these deliveries. No GitHub
merges, original closures, real-data writes or policy activation occurred.
This iteration is PROGRESS; the overall goal remains incomplete.

### Legacy export delivery boundaries inventoried

Re-read the active objective and confirmed the previous iteration was PROGRESS.
Existing verification handles `28480`, `9966`, `79480` and `61137` remained live;
the disposable Rust compiler was consuming CPU, not merely leaving a stale lock.
No build was restarted. Prepared the preflight draft description locally;
publication awaits its complete cache proof so no known-broken offline head is
submitted as a ready delivery. The corrected preflight base has 1,119 SQLx
descriptors; original `8aac739` added 17, and neither preserved preflight source
file has any later original commit changing it.

Inventoried the remaining legacy export commits before the canonical consumers:

- `dc822a0` and `108594e` form one materialized XLSX/PDF boundary: bounded
  materialization, rendering/release authorization, financial/project HTTP guards
  and retained project-resource scope. Include invoice PDF, not spreadsheets alone.
  Reconcile the original snapshot visibility changes with #220's existing
  `pub(crate)` manager helper rather than extracting that shared change twice.
  Reuse the already-recorded prerequisite composition `0046dad` as a candidate
  review base rather than creating redundant shared foundations. Its history
  contains #228 directly and equivalents `41e595d`/`0046dad` for #220/#232;
  those are not the original PR commit IDs, so ancestry alone is not proof of
  equivalence. Check the selected source diff before extraction.
- `dcf4ac8` is a dependent CSV boundary: connection-local cursor, bounded batches,
  release checks and migration 0046's `SECURITY INVOKER` fetch function. Its
  public execution privilege is revoked; separate deployment migration owners
  must explicitly grant the runtime role. Preserve that existing deployment
  contract, and do not introduce real-database migration or grants in this goal.
- The authorization-test files were later changed by canonical commits
  `cbc78a8`, `09bd15f` and `2631186`. Do not copy their final versions wholesale
  into a legacy-only extraction. Preserve both the historical legacy assertions
  and the separately owned canonical additions; account for both in final mapping.

This is dependency/ownership inventory, not a completed source review or a claim
that the export extractions are implemented. Next collect the ongoing preflight
verification/cache, then prepare the first legacy export extraction while the
final Nix gates finish. Original work, data and PR merge state remain unchanged.

### Preflight suite/cache verified and draft published

Corrected verification `28480` exited zero on source `b159184`: 904 app,
180 integration and 160 core tests passed, 1,244 total; 11 existing manual tests
ignored. Complete SQLx preparation then finished successfully. Provenance check
`2054` confirmed all 1,119 base descriptors unchanged, 19 additions identical to
original `db3935d`, 1,138 total, zero removals/modifications/unmatched additions.

Original `8aac739` added 17 descriptors, one already present in this review base.
The three other regenerated descriptors are original queries used by this exact
reader/test slice: inherited repeatable-read fixture setting, fixture policy-zero
update, and the policy-version organization SHARE read. Their original byte
matches were verified; no unrelated implementation is introduced with them.

Published unsigned cache head `ff482c847ab8d14cda15c8a3a56deafbf6acb4a0` and
opened draft [#246](https://github.com/numtide/horae/pull/246). It has 22 files,
947 additions: 547 source/test registration lines plus 400 SQLx lines. The actual
reader remains the unchanged 91-line original. Offline native/WASM lint `86929`
and full clean-head Nix `99711` are live; neither is claimed passed yet. The local
Cargo target is occupied by that lint run.

Old #245 source/cache-head Nix `61137` exited zero on `533922a`, all local
x86_64-linux checks passed, including the inherited browser suite. Its later
test-inclusive head `93f230d` still requires `9966`, which remains live; old
results are not substituted. #240 final-head Nix `79480` also remains live.
PR #245's body records the completed older gate and pending current one.

Reading the retained export contracts clarified two inventory details: the
materialized boundary includes invoice PDF as well as XLSX, and #220 already
delivered the manager helper's `pub(crate)` visibility. The snapshot module
itself remains private in candidate base `0046dad`; account for that remaining
visibility hunk without duplicating the helper change. Use the historical
`dcf4ac8` CSV contract for the legacy extraction, not later canonical refinements
in the final original contract. No export source was changed in this iteration.

Next collect the exact live final-head gates, then extract the materialized
XLSX/PDF boundary with the preserved legacy tests and current master behavior.
The original branches, unpublished work, real data and merge state are intact.
This iteration is PROGRESS; full original ownership and the overall goal remain
incomplete.

### Materialized export extraction and schema dependency correction

Re-read the active objective after the intervening goal-prompt conversation
(that conversation alone made no repository progress). GitHub confirms #216
merged as `02f7b58`. Revalidated live final-head Nix handles `99711`, `9966` and
`79480`; no run was restarted. #240 and #245 have now built their final packages
and reached real browser tests; the full gates remain pending. Recorded the
previously completed offline native/WASM lint `86929` as passed for #246 and
updated its PR description. Its handle is now closed, not an active wait.

Created isolated `fix/materialized-export-authority` from existing review base
`0046dad`, extracting original `dc822a0` and `108594e` together. This is one
materialized-download responsibility, including invoice PDF. Seven complete
files match historical `108594e` byte-for-byte: `reports/limits.rs`, its project
reader and two authorization-test files, `reports/privacy_tests.rs`, and the
two HTTP export-test files. Their later canonical changes remain separately
owned. `reports.rs` retains #220's removal of the old global invoice loader;
its only difference from the historical file is that already-reviewed base
change. The remaining adaptations are export HTTP registrations and snapshot
module visibility. The manager helper was already crate-visible in #220.

Verified #220's snapshot source/test directory is identical to composition
`41e595d`; #232's invoice reader/editor, fee tests, snapshot tests and HTTP
financial checks are identical in `0046dad`. Differences elsewhere are the
known #227/#228 writer coordination, including its invoice/assignment lock test.
The recorded master update changes neither crates nor Cargo manifests/lockfile.

Unsigned initial source `c216440` passed formatting (`17075`, zero changes),
but disposable verification `93167` exited 101: three original test queries
require `organizations.access_revision`, absent from `0046dad`. No test ran or
cache succeeded in that attempt. Do not remove those assertions or substitute
an unrelated organization edit. They verify revision refresh and unchanged
revision across real finalization/editor writes.

Created review-only `integration/materialized-export-prerequisites` at
`3edc0b8be31ffc08acd9a17cfb3022da6b0a62fb`, merging exact #222 `e9695fd`
into `0046dad` without conflicts. This supplies existing non-activating storage
and its pure-domain prerequisite; no new migration or activation is invented.
This local composition is not a delivery merge target. Rebased only the new
extraction using `--no-update-refs` to `abdda6077fc35b76178a7d40bdae4facac9dbaea`.
The extracted readers, tests and HTTP harness remain unchanged by the rebase.
The source delta is 10 files, 2,237 additions and 102 removals; 1,935 added lines
are the four preserved authorization/HTTP test files. Complete suite and SQLx
preparation are now running in `20984` with a private PostgreSQL socket and
invalidated Rust-source timestamps. The reusable local Cargo target is occupied.

Scoped Spec Kit analyze ran the real prerequisite command against original 015,
with no extension hooks. Read the retained spec/plan/tasks plus historical
manager/project export contracts. Coverage within T104–T109:

| Requirement | Tasks | Evidence boundary |
| --- | --- | --- |
| FR-006 | T104/T105/T107/T108 | Current active tenant actor; unchanged legacy manager/project scope |
| FR-007 | T105/T106/T108/T109 | All four materialized HTTP handlers; CSV remains separate |
| FR-010 | T104/T105/T107/T108 | Fresh checks after authority/parent waits and rendering |
| FR-017 | T104/T105/T107/T108 | Exact existing values, no business writes or historical recalculation |
| FR-018 | T104/T106/T107/T109 | Real reader, races, cleanup, size and session-cookie checks |

Five requirements and six tasks mapped (100% bounded coverage); zero unmapped
tasks, ambiguities, duplications or critical/high specification findings in this
slice. SC-006 is a regression subset, not full acceptance. Full canonical US3,
T039/T040/T042, CSV, activation and governance remain open. No original artifacts
were edited or their checkboxes treated as proof of this extraction's tests.

Source review read both complete database authorization-test files and both HTTP
test files. Checked session-derived IDs, pre-query manager authorization,
same-snapshot size/payload, one-statement project materialization, nullable
sentinel handling, private captured IDs, sorted parent locks followed by a fresh
visibility query, body/permit drop on denial, and no DB locks during rendering
or client backpressure. Existing CSV transaction configuration remains unchanged
in behavior. No critical/high source finding identified; executable verification
and complete cache/offline/full-Nix gates are still mandatory and pending.

Next collect `20984`, prove SQLx provenance (accounting for legitimately replaced
legacy descriptors), then run offline native/WASM and final-head Nix before
publishing the scoped draft. Collect `99711`/#246, `9966`/#245 and `79480`/#240
without restarting live handles. The original tracked unpublished work still
exactly matches its backup snapshot; original branches, #208 and real data are
unchanged. This iteration is PROGRESS; the overall goal remains incomplete.

### Materialized exports published; final browser failures corrected

The previous iteration was PROGRESS. Re-read the objective and polled the four
specific live handles; no process was restarted because of an observation timeout.
Corrected XLSX/PDF suite/cache `20984` exited zero: 889 app, 180 integration and
160 core tests, 1,229 passed total with 11 existing manual tests ignored. The
complete SQLx preparation succeeded after all targets were invalidated.

Read-only provenance check `23074` confirmed 1,088 base descriptors: 1,086
unchanged, two removed, zero modified; 49 additions match historical `108594e`
exactly, 1,135 total, zero unmatched. The two removed descriptors are the old
separate project size query (`717614…`) and the project-row projection without
private IDs (`57eb29…`); the single bounded materialization and ID-bearing
projection replace them. No unrelated query was discarded.

Unsigned cache head `d9717e7ed9e9982e01668ee611a9e7aef950c358` is clean and
pushed with review base `3edc0b8`. Opened draft
[#247](https://github.com/numtide/horae/pull/247). Formatting `48059` passed
unchanged. Offline native/all-target and WASM lint `49167` passed on that exact
head, warnings and performance lints denied. Full Nix `32625` is live. The PR
records successful source/cache/lint checks separately from its pending full
gate. No GitHub merge or policy activation occurred.

Two previously live final-head Nix gates exited 1; neither is certified green:

- #240 `79480` on `d2da193` failed in inherited `clients-access.cjs` at its
  `actor.active === true` assertion. `get_me` intentionally no longer returns
  activity. Test-only `a19ea63857dd149a64fc3519a19b3cab7a0489b8` now asserts the
  exact five public identity keys and verifies current activity from the
  disposable actor/tenant row. This strengthens the payload check without
  deleting any role/scope/inactive/anonymous assertion. Focused Chromium run
  `59611` passed all four role/scope scenarios and negative write/session cases
  using the exact failed-head package. Formatting `51138` passed. The fix is
  pushed; full Nix `99785` is running on the new head. The older unbounded
  action-errors interruption remains recorded separately.
- #245 `9966` on `93f230d` failed at the empty-history assertion. Its ARIA
  snapshot showed `Other draft owner` authenticated, while the fixture had
  installed canonical state for `admin@example.com`. The preceding New Project
  permission suite creates the additional Administrator; dev login does not
  promise the seed account. Test-only `14ad9ca` obtains the actual session's
  `get_me` endpoint from the exact built server, validates its IDs, legacy
  Administrator role and DB activity, and creates/cleans that actor's fixture
  only. All history assertions and zero-state preconditions remain. Combined
  browser run `84853` passed `new-project-permissions permission-history` against
  the failed-head package; this exercises the additional-admin prerequisite,
  not just an isolated seed login. Formatting `33741` passed. The fix is pushed;
  full Nix `56923` is live on its new head. Earlier isolated-browser and old-head
  Nix passes do not substitute for this pending complete gate.

#246 final Nix `99711` remains live and has reached browser/deployment tests;
do not restart it. #247's local Cargo lint has finished, so the shared target is
free for the next extraction, independently of the Nix sandbox builds.

Documentation ownership inspection identified exactly 59 changed non-cache
Markdown/JSON paths in original #212: 54 new feature-015 documents, the feature
selector, constitution, New Project spec, AGENTS and README. Feature 015 is not
present in recorded master. Keep these as existing artifacts, not a new specify
exercise. Three histories own the governance/selection/New Project deltas:
`05448a8`, `b3ee8da` and `ab9b1a4`. The constitution remains 1.0.0 on master;
original 1.1.0 is preserved proposal material, not adopted by these extractions.

Master's feature selector now points to 016, and its New Project spec has newer
expense-budget/currency acceptance (including FR-026). A whole-file replacement
from #212 would erase that work. Carry the original permission-transition hunks
onto the newer spec without changing those expense requirements; preserve the
old selector as historical work rather than resetting the current feature.
AGENTS' original SQLx timestamp guidance belongs to cache-preparation hygiene;
README's ICU/name-collision prerequisite belongs to #222's migration deployment
documentation. These paths are inventoried, not yet extracted/reviewed deliveries.
Original progress/quickstart are historical evidence, never proof that an
extracted head passed. Unpublished Clients documents/code remain separate.

Next collect final-head `99785`, `56923`, `99711` and `32625`, and continue the
dependent legacy CSV boundary with migration 0046. Remaining canonical
consumers/editor and documentation ownership are still required. The goal is
incomplete; this iteration is PROGRESS, with no original closure or real-data write.

### Legacy CSV boundary extracted on materialized exports

The intervening prompt-only response did not advance repository state (NO
PROGRESS). Re-read the actual saved objective, AGENTS and constitution 1.0.0,
verified #216 is merged as `02f7b58`, inspected current worktrees/ledger, and
resumed the next safe extraction. Specific final-head Nix handles `99785`,
`56923`, `99711` and `32625` were confirmed live; none was restarted. #246 is
still in its browser matrix, the other three are compiling. These are pending,
not passing gates.

Created `fix/csv-export-authority` in `.worktrees/csv-export-authority` directly
from #247 `d9717e7ed9e9982e01668ee611a9e7aef950c358`. Unsigned source commit
`9f2994d` extracts historical `dcf4ac8`: 13 files, 1,571 additions and 113
deletions, including the complete 815-line database authorization module and
68-line session-authenticated project CSV module. Eleven files match that
historical source byte-for-byte. `reports.rs` additionally preserves #220's
removal of the old global invoice loader; the HTTP harness gains only the nine
CSV route registration lines, retaining its existing legacy-reader and importer
boundaries. Later canonical time/project/grouped export changes are not pulled
into this slice. No dependency, policy activation, UI or business-state write
was added.

Reviewed all production cursor/delivery paths, migration 0046, the complete new
authorization tests, HTTP CSV tests and changed invoice snapshot fixture.
Preserved one reserved close-on-drop connection, explicit READ COMMITTED READ
WRITE, initial authorization before cursor declaration, and a frozen source
snapshot. Each nonempty block reserves output capacity before fresh
organization/actor checks; project blocks sort/deduplicate captured parent IDs
and query access separately after parent-lock waits. Successful rollback and
savepoint release precede synchronous send with no intervening await. A later
denial propagates as body failure rather than successful truncated EOF.

Native input limits remain 1 initial row, then at most 128 rows or the
64 KiB logical-payload crossing row. Output independently flushes first/128/64
KiB records; one oversized record is intentionally allowed, not claimed as a
whole-process memory ceiling. The invoker-only fixed-cursor helper keeps PUBLIC
execution revoked; separate migration owners must explicitly grant their
runtime role. The invoice LEFT JOIN sentinel preserves empty/missing distinction,
nullable fee quantities, stored integer totals and original filename/metadata
across source changes. Error, timeout and cancellation preserve admission and
connection cleanup. No critical/high source finding identified; executable
verification remains pending.

Applied the Rust, async, testing and minimal-change skills. Formatting `10571`
passed with zero changed files; `git diff --check` passed. Suite and complete
SQLx preparation `47215` are running against the helper's fresh private
PostgreSQL cluster, with no TCP listener or real data. The shared local Cargo
target is occupied until that handle finishes; do not launch competing local
Cargo work.

Scoped Spec Kit analyze ran the original feature-015 prerequisite script
(`--json --require-tasks --include-tasks`); required artifacts exist and there
are no extension hooks. Read the relevant current spec/plan/tasks and both
historical and evolved CSV contracts. This extraction uses the T110–T113
legacy boundary only; canonical refinements remain separate original work.

| Requirement | Tasks | Bounded coverage |
| --- | --- | --- |
| FR-006 | T110/T111/T112 | Tenant, active actor and captured-project access |
| FR-007 | T110/T111/T112 | All three CSV source and delivery paths |
| FR-010 | T110/T111/T112 | Fresh post-wait/block checks and revocation |
| FR-017 | T111/T112 | Frozen source, integer values, no business mutation |
| FR-018 | T110/T112/T113 | Negative/race/native/HTTP and regression gates |

Five requirements and four tasks mapped, 100% bounded task coverage, zero
unmapped tasks, ambiguities, duplications or critical/high specification
findings in this slice. SC-006 is a regression subset, not full acceptance.
The unchanged constitution 1.0.0 remains authoritative; the original 1.1.0
proposal and full US3/canonical activation gates are not adopted or completed.
No remediation or original artifact edit was needed by this scoped analysis.

Next collect `47215`, then run `.scratch/verify-cache.mjs` in the CSV worktree
to prove the regenerated cache retains every base descriptor except the two
replaced invoice snapshot-fixture UPDATE queries (`548235…`, `560550…`) and
matches historical `dcf4ac8` for additions. The verifier is prepared but has not
run; expected removals are a hypothesis until verified. Commit the proven
cache, run offline native/WASM and final-head Nix, then publish a scoped draft
with exact evidence and its dependency on #247. Collect the four existing Nix
handles at reasonable intervals. Documentation ownership, canonical consumers,
editor/UI and unpublished Clients remain required. This iteration is PROGRESS;
the overall goal remains incomplete.

### Specification preservation published; preflight browser timeout isolated

The previous goal turn was PROGRESS: source CSV extraction, provenance review,
scoped analysis and the ledger were committed. Re-read the saved objective and
confirmed `47215`, `99785`, `56923`, `99711` and `32625` live. CSV `47215` has
finished compilation and is executing its 916-test app binary; no final suite
or cache result is claimed yet. The shared local Cargo target remains occupied.

Created documentation-only worktree `.worktrees/permission-specification`,
branch `docs/permission-specification`, on master `1b8fa4f`. Preservation commit
`c47199296c755be19023e1c2de8a38c0d148ebe3` contains all 54 original committed
feature-015 documents byte-for-byte from `db3935d` (21,814 existing lines,
including historical research and verification records). No original worktree
or dirty Clients file was copied over or cleared.

Read the Spec Kit analyze instructions and ran the feature-015 prerequisite
script in its original worktree, with no hooks or feature-selector changes.
The extracted committed artifacts, not that worktree's newer unpublished
Clients edits, are the documentation source. Bounded extraction review found
two contextual risks: the absent proposed constitution could be mistaken for
the authoritative version, and historical task/test statements could be mistaken
for current-head verification. These are documentation-provenance findings, not
new product decisions or a claim of full-feature analysis completion.

Follow-up `c77abf9edd5ba7a924ce934b114961af920a5fa4` clarifies the spec, plan,
tasks, research, progress and quickstart. It leaves the full requirement and
task statements unchanged, explicitly keeps constitution 1.0.0 authoritative,
and identifies #218 as the only current separation record. Original logs remain
historical; process IDs in them are not live handles or new next actions.
Applied only the original New Project permission-transition hunks on top of
master's file: new expense inclusion, expense/time currency distinctions and
FR-026 remain intact. No code, schema, cache, design, README, AGENTS or selector
change is included.

Read-only verifier `27861` passed: all 54 preservation blobs match, 48 documents
are still unchanged, six have contextual notes, all 43 functional requirements
and success criteria are identical, and all 236 unique task lines retain text
and state (208 checked historically, 28 open). The exact three original New
Project diff hunks compose over current master, with no other edit. There are
55 changed Markdown paths and zero application/governance changes. Formatting
`56406` passed unchanged; diff whitespace checks passed. Published draft
[#248](https://github.com/numtide/horae/pull/248), confirmed by `54903` exit zero.
Full flake/remote checks and final cross-PR reconciliation remain pending. No
fresh Harvest verification or completed permission implementation is claimed.

All 50 documentation-only commit rows now name #248 for their final feature
document state. The general ownership rule also covers specification hunks in
mixed commits. Original history and discarded intermediate wording remain
recoverable in the verified refs/bundle. Governance/selector exceptions and
the still-unpublished Clients documents are explicit, not silently omitted.

#246 full Nix `99711` exited 1 on `ff482c8`. Its complete browser log at
`/nix/store/2xkgr1s2wbiqpqsfcxlyr50lnc2kz992-horae-browser-checks.drv` shows the
preceding suites passing, then inherited `project-edit.cjs` timing out after
five seconds while reloading the saved project at 390px. The ARIA snapshot is
`Loading project…`, not a denied or wrong-value result. This observation does
not establish the timeout's cause.

Read the full editor test and disposable runner. Unchanged focused sequence
`new-project-transport project-edit`, handle `97035`, passed on the exact failed
package `/nix/store/y6alrw4vmcill044y7r0vj2bz18niw7s-horae-0.1.0/bin/horae`.
It verified 390/768/1440px edit/save/reload, original history and identity,
configured monetary values, dirty-navigation protection, acknowledgement-loss
replay and concurrent-edit recovery. No source, assertion or timeout changed.
Started one complete unchanged-head rerun `41382`; the original failure stays
recorded, its root cause remains unproven, and no full pass is claimed. #246's
PR body now records both results and the pending rerun.

Next collect CSV `47215` and complete its prepared cache-provenance/offline/full
gates before publishing. Collect #246 rerun `41382` and existing final-head
Nix `99785`/#240, `56923`/#245 and `32625`/#247 without restarts. Finish #248's
required checks and reconcile remaining README/AGENTS/governance ownership.
Canonical consumers/editor/UI and unpublished Clients still need disposition.
This iteration is PROGRESS; no merge, original closure or real-data change.

CSV `47215` subsequently completed its workspace suite successfully on source
`9f2994d`: 905 app, 180 integration and 160 core tests passed (1,245 total),
with the same 11 existing manual tests ignored. Complete SQLx preparation is
now running in that same handle/private database. No failure or assertion
weakening occurred. Do not start local offline lint or validate a partial cache
until preparation exits. Ledger formatting `49784` passed unchanged; #246 body
update `78194` exited zero.

### CSV delivery published; deployment and cache documentation accounted for

The immediately preceding response supplied a goal prompt, not repository
progress. Re-read the saved active objective, AGENTS and constitution before
continuing; GitHub reconfirmed #216 merged at `02f7b58`. Revalidated the existing
verification handles rather than restarting them. This iteration is PROGRESS.

CSV preparation `47215` exited successfully after its 1,245-test suite. The
complete cache-provenance check `57325` passed: 1,135 base descriptors, 1,133
unchanged, exactly two obsolete invoice-fixture UPDATE descriptors removed,
41 additions matching historical `dcf4ac8`, zero modified or unmatched entries,
1,174 total. Formatting and unsigned cache commit `1633` completed successfully
as `71232dc4257a0c1831ee9e7005a6a2c1562516ed`; the worktree is clean.

Final-head offline native/all-target and WASM Clippy `68186` both passed with
warnings denied and performance lints enabled. Published draft
[#249](https://github.com/numtide/horae/pull/249), confirmed by `76441` exit zero,
against exact #247 `d9717e7`. Its source/review boundaries and pending checks are
explicit. Full Nix `72947` remains live on `71232dc`; it is not a completed gate.
PR body update `55002` records the completed offline checks. The shared local
Cargo target is free again; the Nix checks use isolated build directories.

Read the README skill, entire #222 README and original migration 0047, and
restored only the original six-line PostgreSQL/ICU/name-collision deployment
note. Commit `516b023780eea04d7b1bf73895c5b325b299c872` is pushed to #222.
`git diff --quiet db3935d HEAD -- README.md` passes: the complete README now
matches the original source. Formatting and whitespace checks passed.
Final-head full Nix `69471` passed all compatible x86_64-linux checks; unchanged
code derivations reused their verified outputs and the new formatting derivation
passed. No migration, runtime code, source worktree or descendant branch changed.
Existing descendants still record their exact `e9695fd` prerequisite; integrate
the documentation follow-up when retargeting, not via needless stack rewrites.

#248 follow-up `49843b240c0179a5142c20fdac7f4e74a8bc323e` preserves the original
seven-line AGENTS cache-preparation guidance. Its whole AGENTS file matches
`db3935d`, verified alongside the prior 54-document/43-requirement/236-task and
three-New-Project-hunk assertions. The extraction now has 56 Markdown paths.
Formatting passed with zero changes, the commit and updated PR body are pushed
(`30390` exit zero). README belongs to #222; constitution 1.1.0 remains explicitly
unadopted and the obsolete selector remains historical. Final-head full Nix
`51945` is running. No application behavior or product decision changed.

#246 complete unchanged-head Nix rerun `41382` exited zero on `ff482c8`, including
the full browser suite. The initial `99711` editor-loading timeout and unknown
root cause remain recorded; no source, assertion or timeout was changed.
PR update `99428` records both outcomes. Prerequisite integration, retargeting
and required remote checks remain separate gates, not implied by this local pass.

#240 `99785`, #245 `56923` and #247 `32625` are confirmed live; their app builds
finished and browser/NixOS checks are progressing. Do not restart these handles
or treat the completed app builds as complete flake checks.

Next collect those three handles plus #249 `72947` and #248 `51945` at reasonable
intervals, recording failures as well as passes. Resolve the remaining canonical
consumer/editor/UI ownership and original test-tooling overlaps from the actual
source dependencies; do not implement unfinished consumers to ease extraction.
Unpublished Clients remains preserved separately, not delivered. Final per-change
accounting, cross-PR integration/review and delivery order are still required.
No merge, original PR closure, real-data operation or canonical activation occurred.

### Editor server extraction and live PR-status checkpoint

Previous iteration was PROGRESS: #249 publication, documentation ownership and
exact-head checks were recorded. Re-read the saved objective and resumed the
existing handles. #240 `99785` on `a19ea63`, #245 `56923` on `14ad9ca` and #247
`32625` on `d9717e7` have now all exited zero with complete compatible local Nix
checks. Their original failures remain historical evidence. #247 body update
`76119` was started; #240/#245 body and table updates still need reconciliation.
#248 `51945` and #249 `72947` remain running, not completed gates.

Read the Rust best-practices chapters 1/2/4/5, async and testing skills and the
Spec Kit analyze procedure. Reviewed the complete original editor API, internal
reader, DTOs, 428-line editor tests, 305-line subject tests, 678-line HTTP matrix
and editor contract. Existing profile/template command implementations already
match the original. No page, CSS, navigation or recovery consumer was extracted.

Ran feature-015 prerequisites with no extension hooks. Bounded analysis maps 13
requirements (FR-002/004/006/010/011/012/013/015/018/025/029/030/032) to ten tasks
(T126–T132 and T139–T141), with no unmapped task, ambiguity, duplication or
critical/high specification finding in that boundary. T126 is already owned by
the command foundation; UI portions of T131/T132 and full T018 remain separate.
SC-006 is only a regression subset, and constitution 1.0 remains authoritative.

Created `.worktrees/permission-editor-api`, `feat/permission-editor-api`, first
on #234. Six complete source/test blobs matched original `db3935d` exactly;
only four module/harness composition points differed. Initial source `ad590e5`
passed formatting, but suite `99212` exited 101 at compilation: the strict
historical `ProfileAudit` decoder used by an original assertion belongs to #245.
An existing dead-code expectation also became unfulfilled after endpoint wiring.
No test was removed, replaced or weakened, and SQLx preparation did not run.

Preserved the initial extraction at `backup/permission-editor-api-before-audit`.
Rebased with `--no-update-refs` onto exact #245 `14ad9ca`, retaining every existing
audit/own/delegation module and test invocation, restoring the full original DTO
and removing the obsolete dead-code expectation. The unsigned corrected source
is `58b0c6f8131221e229f28f2f9a94ff965725f346`; worktree clean, not yet published.
All six preserved blobs still match. The initial rebase continuation attempted
an unavailable signing key; explicit unsigned commit completed the rebase without
changing any original backup ref. Corrected suite/cache verification was started;
collect its live handle before offline lints or validating a partial cache.
Prepared `.scratch/verify-extraction.mjs` expects every #245 cache descriptor
unchanged and every addition matching original `db3935d`; it has not run yet.

The user's status request prompted a fresh GitHub read: 30 separation-related
PRs including this ledger, all draft. Required Flake Check is failed on #239 and
#242, running on #240/#248, and successful on #218/#219/#220/#223/#224/#225/#227/
#231/#238. Numerous Nixbot checks also failed; their causes are not established
by the local passes. Do not describe these PRs as merge-ready or bypass checks.
Next prioritize diagnosis of #239/#242 remote failures alongside collecting
already-running gates, then finish the editor API cache and publication. Remaining
consumer/UI ownership, unpublished Clients and final cross-PR accounting are open.

### Remote failure diagnosis and completed editor source verification

Previous goal turn was PROGRESS: it identified the concrete remote failing tests,
not just a red status. Re-read the saved objective, AGENTS and constitution 1.0;
reconfirmed #216 merged as `02f7b58`. Original refs and worktrees remain intact.

Confirmed GitHub master rules require `Flake Check` and `Format`, plus its squash
merge queue. Nixbot is not a required-status rule, but its failures remain evidence
to classify, not something to discard. Stacked PRs do not receive the master-only
Actions workflow until correctly retargeted. No rule or workflow was changed.

- #239 Actions run `37507170660`, attempt 1, failed in the unchanged
  `new-project.cjs` `chooseField` call: payment terms remained `Net 30` after
  selecting `Custom days` (5-second assertion timeout). The test, menu script,
  selector and New Project implementation have no diff against its master base.
  The asynchronous popover opening focus is a candidate cause, not established
  by this observation. Unchanged focused browser reproduction `66596` is running
  against exact built package `lych8wdz3kwlzjjpc5vavn0v5mxjxqkc-horae-0.1.0`,
  with a fresh disposable database and no real account or mail transport.
- #242 Actions run `37513610808`, attempt 1, failed in unchanged
  `cancelled_page_consumer_retains_lock_until_worker_exits_and_rolls_back`:
  immediate reacquisition returned `ApiImportError::Busy`. Review traced the
  cancellation path through dropped `streaming::run_inner`, worker-held session
  and SQLx 0.8 close-on-drop. That path closes asynchronously, while the test
  assumes a replacement pool slot proves PostgreSQL has released the old session
  lock. The normal completion path explicitly awaits unlock for this reason.
  This is an inherited synchronization assumption; no runtime or test patch has
  been made, and eventual rollback/release has not been independently reproduced
  in the failing schedule.
- #242 Nixbot build 71 is distinct: x86 package build hit a 1,200-second timeout;
  x86 tests failed three durable CSV timing cases and the CLI authorization matrix
  (expected rejection exit 1, received indeterminate submission exit 6).
  Browser/VM checks then report dependency failures. ARM tests passed on that
  build. These logs do not prove all Nixbot failures share one cause.

Started one unchanged-commit rerun of each failed Actions run; both are confirmed
`in_progress`, attempt 2. Preserve attempt-1 evidence regardless of the outcome.
No assertions, timeouts or required checks were removed or weakened. #240 now
also has a passing remote Flake Check. Its and #245's PR bodies now correctly
record final local Nix PASS and distinguish unresolved remote checks; top rows
for #240/#245/#247 were reconciled with their completed handles.

Corrected editor suite/cache handle `97735` exited zero on `58b0c6f`: full workspace
suite passed, then complete all-target SQLx preparation succeeded. Provenance
verifier `37593` passed: six original source files exact, 1,225 base descriptors
unchanged, 14 additions matching original `db3935d`, no removed/modified/unmatched
descriptors, 1,239 total. Formatting passed with zero changes. Unsigned cache
commit is `72e97e16a051360ecaae7fc3dcba3457b6acf1a6`.

Final-head offline native/WASM Clippy `19884` and full compatible Nix `64324` are
running. Existing #248 `51945` and #249 `72947` remain live and have advanced into
browser checks; #249's NixOS tests completed, not a substitute for the full gate.
Next finish editor lint/publication and account its exact original hunks, collect
these existing gates and the two Actions reruns, and retain unresolved inherited
test failures explicitly. Canonical/UI consumer ownership, unfinished Clients,
final per-change mapping and cross-PR integration remain open. No merge, original
PR closure, real-data mutation or policy activation occurred.

### Editor publication and completed remote/local checks

The implementation iteration preceding the LTO question was PROGRESS: #250 was
published, its web lint failure was corrected using existing historical
annotations, and completed checks were collected. The LTO question made no goal
code change. On resumption, the exact live handles were revalidated, not restarted.

#239 run `37507170660` and #242 run `37513610808`, attempt 2, both completed
successfully on the unchanged commits. Required `Format` also passed on both.
Their initial failures remain above, not claimed fixed by a retry. #239's unchanged
complete focused `new-project` browser suite `66596` also passed on its exact
package. No assertion, timeout, runtime behavior or build profile changed.

#248 complete compatible local Nix `51945` passed on `49843b2`, including browser
and VM checks; required GitHub Flake Check and Format also passed on that head.
#249 complete compatible local Nix `72947` passed on `71232dc`, including browser
and both VM suites. Its stack still requires prerequisite integration and
retargeted remote checks. PR bodies for #239/#242/#248/#249 record these results
and distinguish required checks from unresolved Nixbot evidence.

Nixbot #245 build 96 was inspected separately: Darwin VM scheduling lacks
`apple-virt` on the selected builder; ARM Linux VM reports `Shell did not start in time`; Darwin tests fail an inherited credential-retry case with `Socket is not connected` (968 passed, one failed). These specific observations do not
classify every remaining Nixbot failure or certify non-native platforms.

Draft [#250](https://github.com/numtide/horae/pull/250) is published on exact #245
`14ad9ca`. Initial cache-inclusive `72e97e1` passed native lint but WASM reported
17 unused DTO types because the UI consumer is intentionally separate.
The attempted module expectation in `fe7d2f8` was itself unfulfilled (`24445`);
it is removed, not broadly allowed. Final unsigned/pushed
`c82a5b397d61e961b4a336223c5b53412c499b65` restores the eight per-type transport
expectations from original `03e90b1` and the subject-page expectation from
`8d49421`. Their later removal belongs with the preserved UI consumers.

Both final strict lint targets passed (`12387`): WASM first, then native workspace
all-targets with warnings/performance lints denied. Provenance `53103` passed:
five original files are exact; the model is exact after removing only those nine
specified historical annotations, whose source text is checked against the
historical commits. All 1,225 base descriptors and 14 original additions remain
unchanged (1,239 total). Formatting passed with zero changes before commit.
Final diff returns `models.rs` to its base; only four module/harness composition
points remain alongside the preserved files and annotations. No query, test
assertion, serialization field, endpoint semantics or policy activation changed.

Superseded Nix handles `64324` and `22155` were deliberately interrupted after
identifying their exact editor-worktree processes; both exited with interruption,
not success. Final clean-head Nix `90520` on `c82a5b3` is confirmed live. Do not
restart it or count the prior interrupted builds as gates. #250's PR body records
the full failure/correction history, source suite/cache evidence and pending gate.
The top delivery row and eight original-commit ownership rows now account for
the API while explicitly retaining the editor UI and its removed lint annotations.

Next collect `90520` and continue source-based ownership of the remaining
canonical consumers, editor UI, browser tooling and unpublished Clients work.
Final full original-change accounting and cross-PR integration remain incomplete.
LTO was explained to the user but not changed; no merge, original PR closure,
real-data operation or policy activation occurred.

### Build performance priority

The user explicitly requested release-profile optimization and Crane adoption,
in isolated PRs, and authorized merging those build PRs after their checks pass.
That priority precedes the remaining separation work; it does not authorize
merging the extraction PRs or closing the preserved originals. Start from
current `origin/master` (`1b8fa4f`) and retain all existing test gates. Measure
build behavior and dependency reuse rather than claiming an unmeasured speedup.
Resume the remaining ownership mapping above after the build PRs are merged.

### Build PRs published; verification in progress

Both priority branches start from exact master `1b8fa4f`; no extraction branch
was merged or rewritten.

- [#251](https://github.com/numtide/horae/pull/251), `perf/release-thin-lto`,
  worktree `.worktrees/release-thin-lto`, unsigned `0004659`: only two release
  values change (`lto = "thin"`, `codegen-units = 16`). Formatting passed;
  complete native/WASM package `42999` passed (build phase 8m56s). Binary size
  increases from 90,176,360 to 119,730,568 bytes; public bundle allocation from
  3,728 to 3,916 KiB. Exact-master control rebuild `91392` remains running;
  concurrent workloads preclude claiming a controlled speedup. Full local gate
  `57447` and remote Flake Check run `37549267934` remain pending. Format passed.
- [#252](https://github.com/numtide/horae/pull/252),
  `build/crane-dependency-cache`, worktree `.worktrees/crane-dependency-cache`,
  stacks on #251. Pins Crane `47b6b27` without changing Rust/Dioxus/nixpkgs or
  Cargo.lock; separates Dioxus release dependencies from shared development
  dependencies for tests/Clippy/SQLx. No assertions or test execution removed.
  Initial `8be49dc` failed minimal-source construction because generated stubs
  were read-only. Unsigned/pushed `8b4c69d` fixes only that generated directory;
  minimal-source build `59698` passed. Superseded full runs `22361`/`22680` were
  deliberately interrupted, not passed; corrected full gate `10968` is running.
  The first dependency preparation requires fetching/unpacking Crane's vendor
  layout. It is not a warm-cache benchmark.

The initial source-only probe changes an actual Rust file temporarily: application
derivation changes, both dependency derivations remain identical. The probe was
removed with a patch and clean diff verified. Initial tests/Clippy/SQLx also
evaluate to the same development dependency derivation. This demonstrates key
stability, not yet successful Cargo artifact reuse. Final Git-source derivations
on `8b4c69d`: package `qck7vabd696gkqh2898j5qz42c32k3nz`, release dependencies
`1250yiw3cd1ba0yafilansgwfll69x1a`, check dependencies
`45fhbl1nxfzlfayfa2g79rgz64a8gc6i`. Dev shell probe `48847` passed (Cargo1.96.1,
Dioxus0.7.9, database environment present). ARM and cross-overlay evaluation
passed, but cross builds are not certified by evaluation.

Next collect priority gates and baseline, inspect actual dependency reuse and
package/browser/VM behavior, correct findings, then merge #251 only when ready.
Retarget #252 to master afterwards and require its own final checks before merge.
Both remain draft. Original editor gate `90520` also remains running; preserve
its result. Resume extraction ownership work only after the build priority.

### Release-profile local gates passed; protected merge requested

On exact #251 `0004659`, full local `nix flake check` (`57447`) completed
successfully, including browser, deployment VM and OIDC VM checks. Additional
`cargo test -p horae-core --release --locked` (`33471`) passed all 121 tests.
The PR is now ready for review, not draft. `gh pr merge --auto --squash` with
`--match-head-commit 0004659a1d77046a12c18533d68cdcdd3b74e76a` succeeded in requesting
the protected merge workflow (`12844`); required remote Flake Check was still
pending, so this is not evidence of an actual merge. No bypass used.

#252 stays draft on `8b4c69d`. Vendor-only preparation `73582` passed after
parallelizing only the download/unpack derivations; full gate `10968` now builds
both dependency caches. Actual final-application artifact reuse remains pending.
Exact-master timing control `91392` and original editor full gate `90520` remain
live. Do not restart them or count any partial result as full success.

Next collect these runs and #251 merge state. After confirmed merge, rebase the
two own Crane commits onto updated master (preserving the published head with a
lease), retarget #252, and run its required final checks before its authorized
merge. Only then resume the original separation goal. Preserve the measured
binary/bundle size trade-off and avoid an unsupported benchmark percentage.

### Dependency-only WASM packaging correction

The previous priority iteration was PROGRESS: #251 passed full local gates and
entered the protected automatic-merge workflow; #252 was published with separate
dependency caches and a verified source-change key probe. At continuation #251
is still open with remote Flake Check and Nixbot build explicitly in progress.

#252 full gate `10968` on `8b4c69d` failed, not timed out: Dioxus compiled the
minimal source but wasm-bindgen could not find `clone_ref` intrinsics in its
empty WASM program. This then caused the missing `_bg.wasm` packaging error.
Published unsigned `6a05046` adds a web-only Dioxus entry point solely to the
generated dependency source. The real application source is unchanged. It does
not ignore the bundling failure or skip final application compilation. Formatting
passed; corrected full gate `45037` is confirmed live. Cold vendor preparation
is already cached. Actual artifact reuse remains to be verified after it passes.

Separately, original editor gate `90520` terminated with failure in the unchanged
New Project browser test: `chooseField` expected `Hours per person` but received
`Total project hours` after 5 seconds (`new-project.cjs`, scenario near line625).
Its test suite, native Clippy and SQLx phases passed, but this is not a complete
gate pass. The browser test and Projects/select paths have no diff against the
exact #245 base. Cause remains unproven; do not label the failure fixed or exempt
the test. Keep #250 draft and revisit the browser evidence after the build
priority. Exact-master Fat LTO control `91392` is still live.

### Baseline completed and final dependency stub verified for WASM

Exact-master control rebuild `91392` completed successfully, including Nix's
output comparison: native/WASM build phase 18m42s, versus 8m56s for #251. These
are indicative local observations under concurrent load, not a controlled
benchmark or a runtime-performance claim. #251 body now includes both times and
the previously measured size increase. Remote required CI is still pending;
automatic merge remains enabled, not completed. Current branch rules were read:
required Flake Check and Format, squash-only PR merges and protected merge queue.

The intermediate #252 stub `6a05046` failed Rust compilation: its qualified RSX
macro expands to an unimported `dioxus_core` name (`E0433`). No dependency was
added to accommodate a temporary stub. Published unsigned
`6ef7bec7fe8fe6dfbb78a223098de6c51b9d4a1d` uses the existing
`dioxus::prelude::VNode::empty` function directly; the pinned library declares
the exact `fn() -> Element` signature required by `launch`. Superseded run
`45037` was deliberately interrupted after identifying its own PID/worktree,
not reported as a passing gate. Corrected full run `2967` is live; its generated
WASM client compiled and bundled successfully at 37.40s. Server dependency
compilation and subsequent real application/check gates are not yet complete.

The source-only key test was repeated on `6ef7bec` using the same Git-source
flake entry point as CI (not an impure path-flake source). A temporary comment
in `crates/core/src/lib.rs` changed the package derivation from
`v2kkhxzdf7kr837376k8za1y89n7296p` to `gyr6zvcpnvwqhf283r8afb04vfnk8qin`.
Release dependency key `41y1cd29z5bzp58s0anfijd1kvlr0gbx` and development dependency
key `2h9fc5lh0vsbff5yfy09ad77gsr3ngm9` stayed identical. The comment was removed
with a patch; `git diff --exit-code` passed. This still does not claim actual
Cargo artifact reuse until the final package is observed rebuilding from them.

Next collect `2967` and #251 required CI/merge state. After confirmed #251 merge,
rebase all four own Crane commits onto current master and retarget #252, preserving
the published head with a lease. No extraction work resumes before build priority
completion. #250's unresolved browser failure is also recorded publicly in its PR.

### Actual Crane reuse and generated-bundle cleanup

On `6ef7bec`, both dependency builds completed successfully. The final package
restored the release cache and recompiled workspace crates/build scripts only:
real WASM completed in 39.85s and the complete build phase in 3m34s. These remain
indicative local timings under concurrent load, not controlled benchmarks.
Clippy and SQLx passed using the shared development cache. Full run `2967`
continues; partial results are not a full gate pass.

Package review found two JS/WASM pairs: the real application plus the dependency
stub's content-addressed assets. Pinned Dioxus clears its executable directory,
not all restored public assets. Published unsigned `774966e` removes only
`target/dx/horae/release/web` inside the package build sandbox before the real
Dioxus build, preserving Cargo compilation artifacts. No user files or database
state are removed. Formatting passed; final-head full gate `68290` is running.
Verify the final bundle contains no stub assets before marking #252 ready.

#251 Nixbot build112 failed its two ARM VM checks. Both raw logs show missing
KVM, fallback to TCG and guest-shell startup timeout while waiting for PostgreSQL,
before application assertions. The limitation is documented on the PR, not
treated as an ARM runtime pass. Required GitHub Flake Check remains pending;
the exact-head local x86_64 suite passed. No checks or protections were disabled.

Next collect both local runs and the existing #251 required-check watcher.
After actual #251 merge, rebase all five own Crane commits and retarget #252 to
master, then require final-head checks before its authorized merge. Original
extraction work remains deferred until these build-priority PRs are handled.

### Crane final local gates passed; release profile in merge queue

Original full Crane run `2967` completed successfully on `6ef7bec`. More
importantly, corrected full run `68290` passed on exact published `774966e`,
including browser, deployment VM and OIDC VM checks. Package derivation
`59mnq73q7hz4qssknzpc9h3pp7njyf5z` produces
`rbikx1qa8bihgqmcwv38dx200jmih142-horae-0.1.0`. The real build took 3m19s,
WASM31.42s (indicative timing, not controlled). Explicit assertions passed:
both stub asset names absent, real WASM/server present. Only one real JS/WASM
pair ships. Store-reference query returns only glibc, not compiler/cache outputs.

Release/development dependency keys remain `41y1cd29z5bzp58s0anfijd1kvlr0gbx`
and `2h9fc5lh0vsbff5yfy09ad77gsr3ngm9`. Final Clippy, tests and SQLx derivations
are identical to those already verified in `2967`, so their Nix reuse is valid.
The changed package/browser/VM checks were rebuilt/retested. #252 is now ready
for review, with explicit prohibition on merge until rebased master CI passes.

#251 required PR checks passed (Flake Check35m54s, Format44s). Its protected
merge-group run `37552522194` is live, watched every120s by session `37872`.
Format passed; Flake Check remains pending. This is queue entry, not a merge.
The old PR-check watcher `56175` has finished successfully. Both local Crane
gate handles are complete and must not be restarted.

Next collect the merge-group result, confirm actual #251 merge, fetch master,
rebase the five own Crane commits with unsigned commits/known-head lease, and
retarget #252. Verify final-head required CI and preserved local derivations
before requesting its protected merge. Resume the original goal only afterwards.

### Remote Crane platform evidence while protected queue runs

Nixbot build121 on `774966e` completed: native ARM package, tests and browser
passed, as did x86 package, browser and both VM checks. Only the two ARM VM
checks failed. Their exact raw logs each show missing KVM, TCG fallback and
guest-shell startup timeout while waiting for PostgreSQL, before application
assertions. This improves native ARM evidence but is not an ARM VM pass or a
cross-build certification. Results and links are recorded on #252.

Created local recovery ref `backup/crane-before-master-rebase` at exact verified
head `774966e7640bc7338b036b75d6e1f86411faa34b`; the worktree is clean. #251's
merge-group run and watcher `37872` are still authoritatively live. Its earlier
successful PR CI uploaded cache paths successfully; no cache-upload failure was
found. No build was restarted and no merge protection changed. Next action
remains to collect that queue result, not to start new functional work.

### Release merge-group failure and selector test synchronization

The previous iteration was PROGRESS (remote Crane evidence and a recovery ref).
The following wait was verified against live queue watcher `37872`. That watcher
is now terminal: merge-group `37552522194` failed after37m39s. #251 remains OPEN,
with no merge commit and no auto-merge request. Do not restart the old watcher.

The failing browser assertion expected `Hours per task` but retained
`Hours per person` in `chooseField`, New Project scenario near629. This is
not an ARM virtualization failure. Source review found that the helper focuses
its target immediately after opening, before asynchronous native `toggle` can
focus the selected option. A browser-only reproduction with the exact shared
menu script (`.scratch/select-focus-order.cjs` in the release worktree) confirmed
that ordering: native pre-toggle focus was body, an early target focus was then
overwritten by the selected option. Waiting for initial selected-option focus
before choosing avoids it. Probe runs `7549` and `30646` passed; they use no DB.

Published unsigned `a2b12458af4052617bfd5f2110ea27cb8eadf8e3` adds one readiness
assertion and a two-line explanation to the existing test helper. No component,
application behavior, selection assertion, timeout or gate is changed. This is
a necessary test synchronization repair within #251, not evidence that Thin LTO
caused the underlying race. Formatting passed (`31490`). Focused complete New
Project suite `37073` runs against the exact already-built Thin LTO binary with
a disposable database; it has passed the previously failing reload/selection
scenario, but the entire suite is not yet complete. Final-head full Nix gate
`48807` is live. PR body distinguishes old-head successes from new verification.

#252 remains unchanged and fully locally verified on `774966e`; its rebase must
inherit this test repair after #251 actually merges. Next collect repaired-head
gates, then re-enable protected auto-merge with exact-head matching. No merges
have occurred, and the original extraction goal remains deferred until priority
build PRs are integrated.

Focused run `37073` subsequently FAILED at a different readiness assertion:
`Draft saved at` remained `Saving draft…` for5s near New Project1052. Its failure
snapshot already showed the saved status and no pending requests; disposable
server log `/tmp/horae-browser.zCVW0N/server.log` recorded a PostgreSQL pool
acquisition timeout during concurrent compilation. This is not a full-suite
pass and is not yet classified as a product defect or purely load-related.
Do not weaken its timeout. Formerly failing selector scenarios did pass.
Wait for isolated Nix gate `48807`; repaired-head required CI run37556278237 is
watched by live session `16075` at120s intervals (Format45s passed). These two
handles, not the failed old queue or focused suite, are the active verification.

### Verify repaired Crane base in parallel with release CI

The last iteration was PROGRESS: selector readiness correction published with
deterministic focus-order evidence, and the separate draft timeout preserved.
At continuation both `48807` and `16075` are confirmed live. Release application
build and Clippy have now passed within `48807`; remaining gates are pending.

To avoid deferring independent priority verification until #251 merges, rebased
the five owned Crane commits onto repaired `perf/release-thin-lto` now. Published
with exact old-head lease: `587119e5e91d31c615ee63064bced7e43e620112`. Compared
directly with original `774966e`, its tree differs only by the three test lines;
its PR diff relative to #251 remains the same six build files. No goal extraction
or application feature work resumed. Full combined-head gate `6287` is live
with cores2/max-jobs1; previous Crane head results are not claimed for it.

Git's existing `rebase.updateRefs=true` also moved the owned recovery branch.
Immediately restored `backup/crane-before-master-rebase` to exact original
`774966e7640bc7338b036b75d6e1f86411faa34b` using a compare-and-swap update and
verified it. Use `--no-update-refs` on subsequent rebases; do not alter global
configuration. Direct original-SHA comparison, not the temporarily moved ref,
proved the inherited diff. Worktree is clean and PR description updated.

Next collect `48807`, `6287` and required-check watcher `16075`; only request
protected #251 merge after repaired-head verification. Once actually merged,
rebase/retarget #252 onto master with an exact `587119e` lease (or its current
verified successor), no-update-refs and unsigned commits, then require its own
master-base CI. Preserve all timeout evidence and do not weaken browser checks.

### Repaired release head passed full local verification

Full isolated gate `48807` completed successfully on exact
`a2b12458af4052617bfd5f2110ea27cb8eadf8e3`: tests, Clippy, SQLx, browser,
deployment VM and OIDC VM. It passed the formerly failing selector scenario
and all subsequent New Project/draft checks without changing timeouts. The
separate focused run's pool/readiness timeout remains a recorded failure; it
did not recur in this complete isolated run. Do not poll `48807` again.

Requested protected automatic merge again for #251 with exact-head matching
(`83041`), pending required GitHub CI watched by `16075`. The PR body now scopes
the full local success to the repaired head and retains all earlier caveats.
This is a merge request, not a confirmed merge. Crane combined-head gate `6287`
remains active; its native compile was independently observed consuming CPU,
so a quiet log was not treated as a stopped process or grounds for restart.

Next collect merge-request confirmation, required CI, and `6287`. Keep the same
ordering: actual protected #251 merge, no-update-refs unsigned rebase/retarget of
#252, final master-base CI, authorized #252 merge, then original split goal.

Merge request `83041` completed successfully: exact repaired #251 head verified,
auto-merge enabled at2026-10-07T01:37:13Z, stateOPEN, mergeCommitnull. No merge yet.
Crane run `6287` has now built the real package on `587119e`:8m53s, WASM76.88s,
with cores2/max-jobs1 and unchanged restored dependency artifacts. This slower
observation is included in the PR body alongside the earlier3m19s observation:
do not present the fastest sample as a guaranteed speedup. Concurrency/load were
not controlled; artifact reuse, not a fixed timing ratio, is the proven benefit.
Remaining Crane checks continue. Live handles are `6287` and `16075` only.

### Both repaired heads verified; explicit protected queue entry

Previous iteration was PROGRESS: repaired release full local gate passed,
Crane inherited the test synchronization and its real package was built.
At continuation, exact-head remote Nixbot builds125 (`a2b1245`) and128 (`587119e`)
both passed package, tests, Clippy, SQLx, formatting and browser checks on x86 and
ARM Linux, plus both x86 VM checks. Only ARM VMs failed; each of the four exact
raw logs again proves missing KVM, TCG fallback and guest-shell startup timeout
while waiting for PostgreSQL, before application assertions. Both PRs have
current-head evidence comments; no check was disabled or ARM deployment claimed.

Crane full local gate `6287` completed successfully on exact `587119e`, including
all browser and VM checks. Final package `kqhd2w13ls03fp972728ldc36vvr7qsm` was
inspected: no stub JS/WASM, one real pair, server present, runtime store references
only glibc. Required #251 CI `37556278237` passed (Flake34m17s, Format45s); watcher
`16075` is complete. These two handles must not be polled/restarted.

Despite successful normal merge requests, GraphQL showed no queue entry for
#251. Re-read actual branch rules: required Flake Check/Format, squash-only,
ALLGREEN protected queue; no rules changed. An explicit normal CLI request
`89212` still returned no entry. Used the documented GitHub `enqueuePullRequest`
API with exact expected head and `jump:false`, not an administrator merge.
GitHub confirmed position1, stateQUEUED at2026-10-07T01:54:36Z, entry
`MQE_lQDOTRPZ888AAAABHBMpBs4AA_LZzgMiH-I`. This does not establish the CLI's
underlying cause and is not a bypass or an actual merge.

New merge-group run `37559430187` is confirmed active (created01:54:54Z), while
the old failed queue run remains terminal. #251 still OPEN, mergeCommitnull.
Next monitor this new group, then confirm actual merge before the no-update-refs
unsigned Crane rebase/retarget with an exact `587119e` lease. #252's current full
local/remote evidence does not replace required CI on its eventual master base.

### Read-only preparation while the protected queue runs

Merge-group watcher `90784` remains live for run37559430187 at120s intervals;
Format passed and Flake Check is pending. No new extraction or application edit
was started before the two priority build merges. Original #212 remains
`db3935d` with its same18 unpublished paths; recovery references are preserved.

Dependency inventory of the retained readers: directory `1eb13ec`, project-team
picker `6b5dbae`, and initial time reader `4ac30fa` all use the storage loader and
`configure_administration` (already carried by #222/#226) plus
`PermissionRequester` (already carried by #243 and descendants). The project
picker additionally imports the directory's `PeopleCursor`; its authorization
is project-operation scope, not directory access. The directory HTTP test also
contains legacy identity assertions already owned by #240: preserve those and
add only the canonical-reader coverage, rather than restoring an older file.

Do not extract final `time_entries.rs` wholesale as the initial reader: its
later history includes `60f60f9` subject discovery, `5faed76` requester/subject
binding and `75f13a1` report composition. These are separate dependencies to
account for. This is source inventory only, not a completed adversarial review
or newly verified extraction. Continue the protected #251 merge first, then
rebase/verify/merge #252 before resuming extraction work.

Read-only blob comparison against all30 extraction heads (#219–#250, excluding
#229/#230) and master `1b8fa4f` provides a lower-bound conservation check for
the1,214 original changed paths:98 non-SQLx/non-spec files,480 SQLx descriptors
and48 spec files have an identical final blob in at least one extraction.
The other220 ordinary files,341 descriptors and7 spec files require hunk-level
accounting or still-pending extraction;20 original SQLx deletions require
query/cache reconciliation. A differing blob is not evidence of lost work:
several files intentionally contain only one extracted responsibility or have
documented compatibility adaptations. An identical blob is not evidence that
the combined PRs build or preserve behavior. Keep the final ownership and
cross-PR integration audit open; do not convert these counts into completion
percentages. Merge-group watcher `90784` remains live with Flake Check pending.

Queue source identity confirmed: commit `35dc414fa53e43ded274dcb0252c11d44c5d0e74`
has tree `2546cacc3f025ef82c555b13f86126b459c6c34e`, identical to verified
`a2b1245`, with parent master `1b8fa4f`. Its Flake Check remains in progress;
the watcher is live, not a stopped build. Current CI uses pinned Hestia backed
by GitHub Actions cache; [GitHub's cache isolation rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache)
restrict PR-created caches to their merge ref. This is a possible contributor
to repeated queue build time, not a measured attribution or reason to weaken
isolation. No workflow/cache trust changes made. Crane dependency reuse is
verified locally; its eventual master-base CI timing remains unmeasured.

### Release profile merged; Crane verified on its final master base

Protected merge-group run37559430187 PASSED: Flake43m14s, Format53s.
Watcher `90784` is terminal. GitHub confirms #251 MERGED at2026-10-07T02:38:45Z,
commit `35dc414fa53e43ded274dcb0252c11d44c5d0e74`; fetched master matches it.
No rules were bypassed. GitHub automatically retargeted #252 to master.

Preserved current Crane head in `backup/crane-before-release-merge` at
`587119e5e91d31c615ee63064bced7e43e620112`, in addition to the older `774966e`
backup. Rebased only its five owned commits unsigned with `--no-update-refs`
onto actual master; both recovery refs remain unchanged. New head
`f6f09438be109b5dfc376f381f59892921fed736` has an exactly identical tree to
`587119e` and still changes only six build files (83 insertions,26 deletions).
Published successfully with an exact `587119e` lease; worktree clean.

Final-head local full Nix gate `94135` PASSED, reusing the same verified package,
tests, Clippy, SQLx, browser and VM derivations. This validates the rebased
source without claiming that cached checks ran again. Required master-base CI
run37563128636 is now active, watched by `97039` at120s intervals. PR description
updated; automatic protected merge requested with exact final-head matching
(`70881` succeeded). GraphQL confirms it enabled at2026-10-07T02:41:49Z,
stateOPEN, mergeQueueEntrynull and mergeCommitnull. #252 is not yet merged. Next require current
remote checks, confirm a genuine queue entry and its result, then actual merge
before resuming original extraction work. No extraction PR merge authorized.

### Supply the existing signed dependency cache to GitHub CI

Required Crane run37563128636 remained live beyond one hour on `f6f0943`;
Format44s passed. A normal enqueue attempt (`20445`) was rejected because
Flake Check was still in progress: no queue entry or bypass. Partial job-log
download returned BlobNotFound (`68665`), not a terminal build result.

Read-only diagnosis found local Nix already trusts `https://cache.numtide.com`
and its official public key, while the GitHub workflow did not configure it.
The key matches [Numtide's published cache](https://cache.numtide.com/index.html).
Public narinfo confirms both exact dependency outputs (`6264yrq` release,
`p0fga5r` development) and the previously verified `kqhd2w1` package are present
with that signing-key identity. This establishes available reusable artifacts,
not a complete attribution of the old run's duration. No global config changed.

Added eight lines to the two existing install-Nix steps: extra substituter and
extra trusted public key only. Default caches, signature verification, required
checks, job permissions and tokens remain unchanged. Formatting (`67880`) and
pinned-nixpkgs actionlint (`48939`) PASSED. Unsigned/pushed
`170e52175ad0be9b02159ed16f1ca16273af8b52`; PR now changes seven build/CI files.

Current full local gate `12076` is live; it restores the same release dependency
artifact and rebuilds the package because the source includes the workflow.
Current required CI run37568358785 is watched by `90257` at120s; Nixbot138 also
started. All current-head full-gate results are pending, not inherited passes.
Cancellation requested for only the verified superseded run37563128636
(`24771` accepted); watcher `97039` still lives until its terminal result arrives.
Do not restart or count that old run as passed. Collect its final log when
available. PR description distinguishes requested cancellation from completion.

Next collect current local/remote gates and old cancellation, inspect actual
cache reuse in current CI, then use the protected queue after required checks
pass. Do not retry enqueue while the same check is pending. #251 remains merged;
#252 and the original split goal remain unfinished.

Superseded watcher `97039` is now terminal (exit1): GitHub API confirms
run37563128636 completed with conclusion `cancelled`, exact head `f6f0943`.
Its Flake job duration1h11m23s includes cancellation processing and is not a
successful cold-build benchmark. The post-termination log request (`61362`)
still returned BlobNotFound; do not infer a specific compile/test failure or
claim full timing attribution. No forced cancellation or further restart.
PR description updated. Current live handles are local full gate `12076` and
required-CI watcher `90257` (run37568358785, Format50s passed), both on `170e521`.

### Priority build PRs merged; resume the original separation goal

Current-head local full Nix gate `12076` PASSED, including tests, SQLx, Clippy,
browser and both x86 VM checks. Required run37568358785 PASSED on `170e521`
(Flake19m59s, Format50s); watcher `90257` is terminal. Its complete log explicitly
copies both `6264yrq` release and `p0fga5r` development artifacts from
`https://cache.numtide.com` and restores them for the package/test/Clippy/SQLx
builds. Cache reuse in GitHub CI is now observed, not inferred from local runs.
Do not compare this successful duration with the canceled old run as a benchmark.

Final installed package `xm11dbjilz94xnfgpgivxni0vb17daq5` was compared with
previous verified `kqhd2w13ls03fp972728ldc36vvr7qsm`: server bytes and entire
public tree are identical; runtime store references contain only glibc.
Nixbot138 passed native package/tests/Clippy/SQLx/browser/format on both Linux
architectures and both x86 VMs. Both exact ARM VM raw logs (`drv/17/raw`) prove
missing KVM, TCG fallback and guest-shell startup timeout before application
assertions. No ARM VM deployment claim or disabled checks.

Automatic merge entered the normal protected queue without another mutation:
run37569967612 PASSED (Flake44s, Format43s). GitHub confirms #252 MERGED at
2026-10-07T04:09:07Z as `ed558f62ef03401864b0e0228b681cdb352edb62`.
Fetched master equals that commit; its tree exactly matches `170e521`.
PR description and final verification comment updated. #251 and #252 are both
integrated; their branches, worktrees and recovery refs remain preserved.
No live build/check handles remain from this priority work.

Original-goal prerequisite #216 revalidated MERGED at `02f7b58`. Original #212
is still `db3935d` with the same18 unpublished paths; #250 remains clean at
`c82a5b3`. No extraction PR was merged or closed. The original goal is active
and incomplete. Next resume #250's inherited browser-gate reconciliation against
the now-merged selector repair/build configuration, using a preserved isolated
prerequisite composition so its review diff stays scoped. Then continue the
inventoried reader/editor/consumer groups and final hunk-level ownership plus
cross-PR integration audit. Do not treat build optimization as completion of
permission separation, activation or Harvest parity.

### Editor API verification on the merged build base

Revalidated #216 as merged and #250 as open/draft at `c82a5b3` with a clean
worktree. Preserved that exact head as
`backup/permission-editor-api-before-build-base`. Created review-only base
`integration/permission-editor-build-prerequisites` at `0117991`, combining
master `ed558f6` and prerequisite #245 `14ad9ca` without conflicts. Relative to
#245 it changes only nine inherited build/CI/browser-helper files (116 additions,
39 deletions); no permission source was replaced. This branch is a verification
composition, not a delivery target or authorization to merge its prerequisites.

Rebased only the four editor API commits with `--no-update-refs`; new head is
`c727bc80608a61dc5e138c39a2fc389c452425b5`. All four entries in `git range-diff`
are equivalent. The review diff remains 24 files, 2359 additions and 16 deletions.
Published the prerequisite branch and the owned rebase with an exact old-head
lease, then retargeted #250; push/retarget handle `5584` completed successfully.
No extraction PR was merged, no original source was edited, and no policy was
activated.

Full current-head `nix flake check -L --cores 2 --max-jobs 1` is running in the
editor API worktree as session `46207`. Its result is pending: old-head passes
do not certify this composition, and the merged select-helper repair has not
yet been proven to close #250's browser failure. Next collect this gate and
reconcile the result, retaining the draft until verified; continue the existing
reader/editor/consumer inventory without expanding product scope.

### Scoped people-directory reader extracted independently of editor operations

Previous iteration was progress: #250's owned commits were preserved, rebased
without patch changes, published and submitted to a new exact-head gate. Its
session `46207` remains live; formatting and WASM package compilation passed,
but the full gate is still pending. No restart or inferred terminal state.

Created `feat/scoped-people-directory` in `.worktrees/scoped-people-directory`
from the existing review-only base `0117991`. Draft PR #253 contains original
reader commit `1eb13ec`'s production endpoint, DTO, storage reader, seven DB
tests and registered-session assertions. It does not require #250 editor
operations or #240 legacy projections. Shared dependencies are permission
storage/grants, `configure_administration`, person-management relationships,
profile test fixtures and the existing `PermissionRequester` value in the base.
No migrations, UI consumers, legacy `list_users` changes or activation are added.

Production reader/DTO and DB test files are byte-identical to `1eb13ec`.
Exact string comparison also passed for the complete session endpoint and HTTP
test body. Only the HTTP module/entry name changes: canonical `check_scoped`
becomes `scoped_directory::check`, registered in the existing HTTP suite. This
avoids copying the legacy assertions already owned by #240. The later
`ee16165` DTO changes are only an import spelling and removal of two web-only
lint expectations; those stay with its unextracted UI consumer. Specification
history/contracts remain owned by #248, not duplicated into this code PR.

Adversarial source review checked session-only actor/tenant derivation, explicit
policy-1 admission, active actor SHARE locking, fail-closed corrupt/missing
permission state, managed-person rather than project-manager scope, scope before
activity/cursor/limit, fixed identity-only projection, safe error translation,
bounded transaction settings and cancellation. The existing seven tests cover
revocation waits, relationship removal, direct deactivation, pool reuse, deleted
cursors, page boundaries and foreign/empty scopes; HTTP assertions cover forged
authority, unauthenticated/revoked callers and non-disclosing errors. These are
source-review observations, not yet a passing execution or cross-PR integration
claim. No new product semantics or relaxed assertions.

Initial unsigned head `269e58a` passed formatting (`68456`, zero changes).
Verification `14669` then FAILED offline compilation for two original test
queries whose descriptors came from earlier commits `03e90b1`/`8d49421`, not
the directory commit itself. Preserved and copied those exact source descriptors
(`0c02f33`, `f68ed67`) in unsigned/pushed `3a3753826a17923ecb1464aa9d911876e5a9ef86`.
The PR now adds 23 files/1177 lines, including 14 original SQLx descriptors;
two other descriptors added by `1eb13ec` already exist in the base. No base
descriptor was removed or modified.

After `14669` terminated, restarted tests/Clippy/SQLx verification on the changed
head as session `18891`, using isolated disposable PostgreSQL in Nix. Results
remain pending. Full browser/deployment gates and composition with #240/#250
remain required. Next collect both live gates, resolve only demonstrated
extraction failures, then continue the project-team reader/editor UI groups.
Original #212 still has its same 18 unpublished paths untouched; #208/#217 and
all original backups remain preserved. No extraction PR merge or closure.

### Project-team identity reader extracted on the shared cursor prerequisite

Previous iteration was progress: #253 was published with its source ownership,
preservation proof and a demonstrated cache-dependency fix. Current #253 head
`3a37538` now PASSES strict server/core Clippy and live SQLx cache checking in
session `18891`; 189 core tests passed, application tests are still compiling.
Session `46207` on #250 `c727bc8` remains live: package server/WASM and Clippy
passed, browser suite is progressing. Neither full gate has finished.

Revalidated #216 MERGED before creating `.worktrees/project-people-picker`,
branch `feat/project-people-picker`, from #253 `3a37538`. Unsigned/pushed head
`f498c3f93254cf87dc026b0e0a10c8bd4713d141` is draft PR #254, based on #253 for
the existing `PeopleCursor` type and inherited permission foundations. It does
not require editor API #250 or legacy projection #240. No new abstraction was
introduced just to remove that existing dependency.

The four complete original DTO/reader/DB-test/HTTP-test files are byte-identical
to source commit `6b5dbae`; exact comparison of its complete session endpoint
also passes. Adaptations are only module registration and test wiring beside
already extracted modules. Ten original SQLx descriptors are preserved unchanged.
The diff is 19 files/1258 additions, with no modifications to legacy operations,
no UI, no assignment writes, no migrations and no activation. The three DTO
web-only lint expectations removed by `2497dbe` stay until that original UI
consumer is extracted; no runtime-content difference is hidden there. Contracts
and historical specification changes remain owned by #248.

Adversarial source review checked separate create/edit grants, current managed
project designation, tenant-bound project existence, failure on missing/corrupt
authority before query validation, active-only candidate filtering, literal
substring search rather than wildcard expansion, 50-row pages/51-row lookahead,
100-character search and 500-ID resolution limits, ID/name-only output, session
requester binding, bounded transactions and cancellation. Original archived
project behavior is preserved, not reinterpreted. The nine unchanged database
tests exercise scope, page boundaries, foreign/deleted cursors, unconfigured
candidates, revocation/deactivation waits, authority held through materialization
and cancellation/pool-default restoration. Original HTTP tests retain forged
authority, unauthenticated/denied callers, safe errors and resolution checks.
Source review is not a substitute for execution or full integration.

Formatting `66398` PASSED with zero changes. A read-only inventory matched all
47 SQL macro invocations in the new reader/DB/HTTP files to exact cached query
strings, with zero missing descriptors; this is not a compile/schema test.
Runtime, server/WASM and full Nix gates for #254 have not started yet, while
the two existing verification sessions remain live. Next collect #250/#253,
then run #254's gates and cross-PR compositions before moving to the retained
time-reader/editor/UI blocks. No merge or closure was requested or performed.

### Cross-PR composition preserves legacy and canonical read coverage

Previous iteration was progress: draft #254 and its source ownership were
published. Created isolated `.worktrees/permission-readers-editor-check`, branch
`integration/permission-readers-editor-check`, from #254 `f498c3f`, then combined
#250 `c727bc8` and #240 `a19ea63` in unsigned local commits `a9f374e` and
`7a2d61c1fa6513a4c10056ef94f1f2cc6c2721c9`. This is a published verification
branch only, not another delivery PR, a GitHub PR merge or a master change.

Conflicts involved module/test registrations and imports, plus adjacent legacy
directory replacement context. Kept every module and HTTP test call from both
sides. Kept #240's reviewed `UserListItem` projection and removal of obsolete
`hide_rates` code/test, plus #253's full canonical endpoint. The resulting entire
`server_fns/users.rs` is byte-identical to original `1eb13ec`, not an invented
resolution. Dedicated reader/DTO/DB/HTTP files match #253/#254 unchanged; editor
files match #250 unchanged; legacy directory/session/approval-label HTTP files
match #240 unchanged. The three source heads are verified ancestors. Formatting
`94586` passed with zero changes, clean worktree; push `41158` completed.

#253 exact-head tests/Clippy/SQLx session `18891` is now terminal PASS. All seven
directory DB tests and the registered-session HTTP suite passed. Application
unit result: 976 passed, zero failed, 11 inherited ignored; core189 passed,
zero skipped. Additional test binaries passed (7, 5, 1, 44, 35, 36, 45, 11,
5, 1 and 8 tests). No tests were waived or modified in this extraction.
Standalone browser/deployment checks still remain; cross-PR evidence cannot
silently replace the exact standalone-head gate. PR description/comment updated.

With #253's compilation finished, started full Nix check for #254 `f498c3f`
as `81722`; it restored the exact package from Numtide cache and is executing
the remaining checks. After `18891` finished, started combined tests/Clippy/SQLx
on integration `7a2d61c` as `24884`. Both results remain pending. #250 full gate
`46207` is still live; its browser now passes the prior New Project per-person
budget selection and proceeds through the remaining responsive cases. This
does not yet certify the complete browser or full Nix suite.

Next collect `46207`, `81722` and `24884`, inspect any demonstrated failures,
then close the standalone #253 browser/deployment gap and continue the remaining
original time-reader/editor/UI extractions. Keep these PRs draft and no merges.

### Standalone directory full gate completed

Previous iteration was progress: published the isolated cross-PR composition and
completed the directory tests/cache/lints. Re-polled the same live handles this
iteration rather than restarting any build. #250 `46207` completed its full
browser derivation successfully, including the previously failing New Project
selection; SQLx passed and its test derivation is now running. Combined `24884`
passed Clippy and continues live SQLx checking. Picker full gate `81722` is still
live. These are partial results, not full-gate passes.

Started `nix flake check -L --cores 1 --max-jobs 1` on unchanged #253 `3a37538`
as `2803`; it completed with exit0, rebuilding only current formatting while
recognizing available cached check outputs. The subsequent read-only local
`nix path-info` request `66589` exited1 because three cached outputs were not
yet present locally; this was not a failing browser or deployment test.
Explicit `nix build` of browser/e2e/e2e-oidc then PASSED, materializing all three
exact outputs from the signed Numtide cache:

- `/nix/store/gayvrydx0rbm7pgq9b50fyhiggbj2w61-horae-browser-checks`
- `/nix/store/l4vka8grd4j3m54xr921q95v8vkfbivl-vm-test-run-horae-e2e`
- `/nix/store/asch5fnxcb7hr3k644akjnal97cd642m-vm-test-run-horae-e2e-oidc`

This closes the exact standalone-head browser/deployment gap by verified cache
reuse, not a claim of fresh local execution. Working tree is clean at the same
head. PR #253 description/comment and source ownership row updated. Cross-PR
integration remains pending, so draft status and prerequisite/retarget conditions
remain unchanged. Next collect `46207`, `81722`, `24884`, then complete combined
browser/deployment verification and continue the unextracted original groups.

### Delivery index reconciled with completed evidence

Previous iteration was progress: #253's full native gate and exact signed cached
browser/deployment outputs were verified and published. This iteration updated
the primary delivery table, which still showed #250's old base/failure and
omitted #253/#254/the combined check branch. It now records current heads and
bases, completed checks versus live gates, and the preserved failure history.
Also corrected the original `dcf4ac8` ownership row: #249's full Nix `72947`
passed earlier, as already recorded in its delivery row and completion entry;
no old check was rerun or reassigned to a different head.

#254 `81722` has now passed all nine project-people DB tests, the inherited
seven directory DB tests and the combined registered-session HTTP suite. Its
application unit result is 985 passed, zero failed, 11 inherited ignored;
additional test binaries passed. The full gate is still live in browser checks.
#250 `46207` and combined `24884` remain live in their test phases after completed
Clippy/SQLx; neither is restarted or counted as a full pass.

Read-only next-boundary inventory reconfirmed the original time-reader lineage:
`4ac30fa` adds the scoped reader/DTO/DB and HTTP tests; `60f60f9` adds subject
discovery; `5faed76` binds page reads to requester/subject. Later `e1ddd9a`,
`48a6533` and `02c4245` modify the same DTO for consumer/command contracts, and
`75f13a1` modifies the reader for report composition. Do not transplant the final
whole files as the initial-reader extraction. `c80233b`'s inserted legacy
invoice-identity HTTP block and model change already belong to #242 (`43337fc`)
and its independently registered `time_entry_payload` test; preserve its
canonical fixture remainder without duplicating that legacy test. No new
reader branch, runtime code, schema or product behavior was changed this turn.

Next collect the same three live handles, finish combined browser/deployment
verification after its tests, then extract the remaining original reader/editor
and consumer groups with this ownership boundary. Full original hunk accounting
and final cross-PR acceptance remain open; no original PR or extraction is merged.

### Initial scoped time reader extracted; two full gates completed

Previous iteration was progress: the delivery index was reconciled and the
time-reader lineage isolated without importing later consumer changes. Confirmed
#216 remains merged at `02f7b58` before editing. Read the goal, constitution and
Rust/async/testing/Ponytail guidance; no new product decision or abstraction.

#250 `46207` completed with exit0 on exact `c727bc8`: full native Nix, including
browser, deployment and OIDC, passed. Application unit result: 982 passed,
zero failed, 11 inherited ignored; core189 and additional binaries passed.
#254 `81722` also completed with exit0 on exact `f498c3f`: full native Nix passed,
including its complete browser run and exact cached outputs where available.
Application unit result: 985 passed, zero failed, 11 inherited ignored; all nine
picker DB tests, seven inherited directory DB tests and real-session HTTP
assertions passed. Updated both PR descriptions, preserving old failure history.

Combined tests/Clippy/SQLx `24884` completed with exit0 on unchanged `7a2d61c`.
Started its full `nix flake check -L --cores 2 --max-jobs 1`; process1004924 was
confirmed live in `.worktrees/permission-readers-editor-check`. The launch
response was truncated before retaining its session handle; do not restart or
interrupt it. Collect exact derivation/log evidence after that process ends.
This combination does not yet include #255.

Created `.worktrees/scoped-time-reader`, branch `feat/scoped-time-reader`, from
review base `0117991`, and published draft
[#255](https://github.com/numtide/horae/pull/255) at unsigned commit
`d93e1af639db34b3d709f7b6917d459f31dfb885`. Scope is the initial `4ac30fa` reader,
not the final mixed Timesheet/report file. Four whole files (DTO, transaction
reader, 576-line/eight-test DB suite and 155-line session HTTP suite) are
byte-identical to `4ac30fa`; the 31-line endpoint is copied unchanged. Only five
module/fixture composition points are adapted. No directory/editor/picker
dependency is needed. No migration, UI, command or activation is included.

Restored the original16 SQLx additions. A static48-macro inventory identified
seven older descriptors reused by these source files but absent from this base;
all seven were copied unchanged from the `4ac30fa` tree before compilation.
An initial scanner misparsed the quoted raw SQL literal; corrected the scanner
and discarded that false positive, with no source query edit. Final extraction
is32 files/1473 insertions, including23 SQLx descriptors and the nine Rust paths.
No original source or local unpublished work was removed.

Bounded adversarial review checked session-derived authority, tenant joins,
own/managed/all scope before filters/limits, explicit non-financial projection,
descending exclusive pagination, actor/policy gates, revoke/cancel transaction
tests and sanitized errors. No critical/high finding in this boundary. Preserve
the query/page web-only lint expectations until the real UI consumer arrives.
Later discovery `60f60f9`, context `5faed76`, consumer/command/report changes remain
unextracted; legacy invoice payload cleanup belongs to #242, not #255.

Formatting `53706` passed with zero changes, and committed-tree formatting passed
again in full native Nix `40092`, which remains live on exact `d93e1af`; it has
restored the merged Crane release dependency artifact and started app builds.
Compilation/test/browser/deployment success is not yet claimed for #255.

Next collect #255 `40092` and combined full-gate evidence, correct any demonstrated
failures, then compose #255 and continue original subject-discovery/context and
editor/UI extractions. Retargeted gates and final original-hunk accounting remain
open. No extraction PR or original PR was merged or closed.

### Timesheet person discovery extracted on its real prerequisites

Previous iteration was progress: published #255 and recorded the completed
#250/#254 gates. Re-read the objective, AGENTS, constitution and required skills;
confirmed #216 merged at `02f7b58`. Polled #255 `40092`, still live. The previous
combined full check process1004924 was also confirmed live; neither was restarted.

Created review-only branch/worktree `integration/scoped-time-people-prerequisites`
at unsigned merge `40102aed68dde211cc4f0c13b83e6dda04c78133`, composing exact #255
`d93e1af` and #253 `3a37538`. Both ancestor checks passed. The two conflicts were
only HTTP and DB module registrations; retained both readers and both suites.
This is not a GitHub PR merge or delivery destination. Published the branch for
the next scoped diff; no editor API, legacy identity projection or project-team
picker dependency was added.

Extracted original `60f60f9` into `.worktrees/timesheet-people-discovery`, branch
`feat/timesheet-people-discovery`, unsigned commit `1552fdb`, published as draft
[#256](https://github.com/numtide/horae/pull/256). It uses #255's admission
transaction and #253's existing `PeopleCursor`, avoiding a duplicate cursor or
unnecessary abstraction. Scope is identity discovery under time-read authority,
not a new directory grant, UI, write capability, page-context contract or policy
activation. The original active-participant/retained-history behavior is intact,
including managed-project members who have no hours in the selected period.

Three whole DTO/reader/711-line DB-test files match `60f60f9` exactly. Eight DB
tests, the 29-line endpoint and 135-line HTTP addition are unchanged. The combined
HTTP file differs from the original only by the36-line legacy invoice-identity
block already owned by #242; no assertion from `60f60f9` was removed. Five SQLx
descriptors are original. Total owned diff:11 paths,1105 additions/eight deletions;
deletions are the original admission-helper extraction, not removed checks.
Specifications remain in #248 and no duplicate specification package was created.

Bounded adversarial review checked tenant/activity fences, canonical grants
versus legacy roles and directory authority, minimal identity projection,
scope-before-search/filter/page, stable cursor behavior, malformed parents,
unrelated-hour exclusion, revocation waits, cancellation and error sanitization.
No critical/high finding in this boundary. Formatting `44568` passed with zero
changes. Static inventory matched all61 SQL macros in the reader, discovery
tests and HTTP suite to exact descriptors, with no missing query. This does not
replace database schema/type verification.

Started tests/Clippy/live SQLx on exact `1552fdb` as `61768`; Clippy is currently
running after restoring Crane check artifacts. Derivations are
`1ac19fafdp98kvyrmjqlfma56cbbz9j4-horae-tests-0.1.0`,
`gmqf2f49p879pssv08wqaxprnzj3c809-horae-clippy-0.1.0` and
`l3y7narrmiyf92z0fzczpm086fr4k66b-horae-sqlx-prepare-0.1.0`.
No runtime/full-gate pass is claimed yet. #255 `40092` remains live after its
successful client build; no failure or terminal result was observed.

Recovered exact prior combined `7a2d61c` derivation identifiers by evaluation:
browser `pypjghkk1hhc8raifxrlrhk0p45dvj6z`, e2e
`r8x7fg3dy674cjxzsjqz7yizqknaib1d`, OIDC
`222bzhk6jjk8cxijg1vd3j9cip16qk2r` (all `.drv`). Its three outputs were not yet
valid when queried (`nix path-info` exit1); that observation is pending build
evidence, not a failed check. Preserve the live process and recover its exact
logs/outputs before claiming completion. Native resources remain sufficient
(51GiB available RAM and108GiB free disk); no cleanup performed.

Next collect `40092` and `61768`, recover combined full-gate completion, run the
remaining standalone/full integration gates, then extract original `5faed76`
requester/subject context and the pending consumer/editor/UI groups. Continue
original-hunk accounting; no original PR or extraction was merged or closed.

### Requester-bound Timesheet context extracted

Previous iteration was progress: published #256, its explicit combined base and
source ownership. Re-read the objective and applicable skills, confirmed #216
remains merged at `02f7b58`, and checked the clean published source base before
creating the next worktree. Original source worktrees remain untouched.

Published draft [#257](https://github.com/numtide/horae/pull/257), branch
`feat/timesheet-page-context`, worktree `.worktrees/timesheet-page-context`, at
unsigned commit `8e09e6088eb68d253ec9bbe7cfe033b38e5e019c` on #256 `1552fdb`.
This extracts original `5faed76`: requester/policy-bound page loads resolve an
active selected person and their scoped rows under one transaction. Explicit
legacy-own policy is preserved; canonical denial never selects legacy mode.
There is no connected UI, write command, policy activation or data migration.

Three whole DTO/reader/354-line DB-test files match `5faed76` byte for byte.
All six DB tests and the33-line endpoint/109-line HTTP additions are unchanged;
only the new test-module registration is adapted. The HTTP suite still excludes
exactly the36-line legacy invoice-identity block independently owned by #242.
Total owned diff is six Rust paths,713 additions and33 deletions; deletions are
original helper extraction and transaction ownership changes, not removed tests.
No new SQL descriptor is needed: static inventory matched all37 macro query
strings in the reader, context DB tests and HTTP suite to existing descriptors.

Bounded adversarial review checked session-derived identity, expected requester
and policy mismatch, active-subject lock lifetime, per-page reauthorization,
scope narrowing, legacy-own restriction without canonical fallback, foreign and
inactive subjects, date/cursor validation, archive races, cancellation and error
sanitization. No critical/high finding in this boundary. Historical DTO web-only
lint expectations remain until the actual Timesheet UI consumer is extracted.
Formatting `17124` passed with zero changes and source/whitespace checks passed.

Started Nix tests/Clippy/live SQLx on exact `8e09e60` as `63162`, currently in
Clippy after restoring Crane check dependencies. Derivations are
`v02bmlf0gh6plfblz53vv1ca9m1h0z15-horae-clippy-0.1.0`,
`w2k6hrywi6rf5pfpi4kajgns6wy3mpxi-horae-tests-0.1.0` and
`xi4ji136irf6wp9rcfc1pcb35barknp9-horae-sqlx-prepare-0.1.0` (all `.drv`). No
runtime/full-gate pass is claimed for this new head.

Existing verification progressed without restart: #255 `40092` passed release
server/WASM and Clippy and is now running browser checks. #256 `61768` passed
Clippy and live SQLx and is compiling the application test suite. The earlier
combined `7a2d61c` full-check process1004924 remains live; its package build log
reached fixup and the exact browser log had no result yet. None of these live
full gates is counted as complete.

Next collect `40092`, `61768`, `63162` and combined exact-output evidence; finish
standalone and cross-PR full gates, then extract the original connected Timesheet
consumer (`e1ddd9a` plus its relevant later fixes) and editor/UI groups. Retain
the original14-line removal of two DTO lint expectations with their real consumer. Original-hunk
accounting remains incomplete; no extraction/original PR was merged or closed.

### Combined reader/editor verification closed and preservation audit refreshed

Previous iteration was progress: published #257 and preserved its original
contracts/tests. This iteration performed read-only verification and inventory;
no runtime code or original worktree changed. Revalidated all35 extraction PRs
#219–#257 (excluding #229/#230 and merged priority #251/#252): all remain open
and draft at their recorded heads. Original #212 still has the same18 unpublished
paths; original recovery refs remain intact.

#256 `61768` completed with exit0 on exact `1552fdb`: tests, Clippy and live SQLx
passed. Application unit result:992 passed, zero failed,11 inherited ignored;
all additional test binaries passed. Started full native Nix `37414` on the same
head; it remains pending. #257 `63162` passed Clippy/live SQLx and core189 tests
and continues the application suite. #255 `40092` remains live in browser checks.
These partial results do not certify either remaining full gate.

The original combined full-check process1004924 terminated. Its exact browser
output was then valid; on unchanged clean `7a2d61c`, full native Nix `16434`
completed with exit0, reporting zero rebuilds and all checks passed. Verified
the browser/e2e/OIDC output paths against their expected derivations and read
their terminal logs: browser finishes permission history, deployment script
finished in80.80s and OIDC in26.01s. This closes that combination's full-gate
evidence without claiming the cache-verification invocation reran those tests.
#250/#253/#254 descriptions now reflect the completed integration; retargeted
gates and later #255–#257 composition remain separate requirements.

Read-only conservation comparison used the35 live GitHub PR heads and merged
master `ed558f6`, against original `db3935d` from `9301112`. At the same path,
106 ordinary files,520 SQLx descriptors and48 specification files have exact
final original blobs in at least one extraction or master. The212 ordinary,
301 SQLx and7 specification files without exact matches still require original
hunk accounting or pending extraction; intentional adaptations are not losses.
Compared with the earlier extraction set, #253–#257 add eight exact final
source/test files and40 exact final SQLx descriptors. These are conservation
lower bounds, not completion percentages or integration/behavior proof.

The audit disabled rename detection to include both sides of replacements:
1,240 physical changed paths include46 removed SQLx paths. Confirmed Git's
rename-aware view still has1,214 changes, with26 SQLx rename pairs and20 unpaired
deletions, explaining the count difference exactly. Preserve old-query removal
and new-query adoption together during the final cache reconciliation; no cache
was deleted or rewritten by this inventory.

Timesheet history establishes the next delivery order: `48a6533` and `02c4245`
define/implement person-bound commands before the final navigation/action
consumer `a0632a8` and weekly submission `b8b1c60`. The initial read consumer
`e1ddd9a` and later focus/person-switch/date-offset fixes (`68bbaae`, `e29f4d8`,
`5f7895c`) belong with the corresponding connected UI, not the server command
PR. Browser fixture changes from `2497dbe` cross the project editor and must be
accounted separately. The preserved delegated-command branch `a0ea691` already
contains patch-equivalent command commits; do not extract a second copy from it.

Next collect `40092`, `37414` and `63162`, finish #257's full native gate, then
compose the later readers and extract original person-bound commands before
their final UI consumer. This refines the preceding next-action order based on
actual commit boundaries; it does not reduce scope. Full hunk accounting and
remaining original editor/report/project/task/UI groups remain open. No merge,
closure, activation or real-data change occurred.

### Timesheet reader gates closed; person-bound command extraction started

Previous iteration completed combined reader/editor verification and refreshed
preservation accounting. This iteration closed all three later standalone
reader gates without changing their heads: #255 `40092` on `d93e1af`, #256
`37414` on `1552fdb`, and #257 full native Nix `72055` on `8e09e60` each
terminated with exit0 and all checks passed. #257 tests/Clippy/live SQLx
`63162` also terminated with exit0:998 application tests passed, zero failed,
11 inherited ignored; core189 and all additional test binaries passed.
The full #257 invocation reused available outputs and rebuilt its format check;
it is not evidence of independently rerunning every cached test. Cross-PR
composition with the editor, project picker and legacy writers remains pending.

Created isolated `feat/timesheet-person-commands` at #257 `8e09e60`, worktree
`.worktrees/timesheet-person-commands`. Extracted the original67-line contracts
from `48a6533` together with the implemented35-line endpoints, complete command
module and four test files from `02c4245`. Kept existing UI-only lint expectations:
their original removals depend on the later connected UI, not these endpoints.
Registered the original real-session HTTP tests with the existing harness.
No UI consumer, policy activation, migration or new behavior is included.

Static review confirmed existing requester/policy binding, active same-tenant
subjects, own/person/project/all write scope, both ends of project moves,
atomic bulk sets, approval/billing conflict boundaries, task eligibility,
owner-only terminal timer recovery, revocation/cancellation fences and
post-commit effects. Approval-covered editing remains explicitly incomplete
in the original implementation and must stay declared, not silently enabled.
The new path uses the existing organization SHARE and subject write barriers;
#241's legacy-writer changes are a composition check, not a missing symbol.

Copied38 original command SQLx descriptors. Static inventory of84 macro calls
found one older reused descriptor absent from the extraction base:
`d0fb58934201c864ceeb74648fc7516252742592f57cee657d5c81ca56c67621`
(`SELECT minutes FROM time_entries WHERE id=$1`). Restored it byte-for-byte
from the same original tree; no query was changed. Whitespace check passed.
Initial staging orchestration `8929` timed out in automatic approval before
execution; the permitted retry succeeded. Formatting `62818` passed with zero
changes. All44 whole command/test/cache files match their original blobs.
Published unsigned `d2b45cadecae68bbcb28dfd4250462bcbd194e41` as draft #258,
stacked on #257;47 paths,2844 additions, including39 SQLx descriptor paths.
Nix tests/Clippy/live SQLx `64524` is running in Clippy. Exact derivations:
`jdf7ladl6xv4z3dx03rd3qc4gh7n8s5a-horae-clippy-0.1.0`,
`869wwixbiws3vpq7qisbs4b77iq0lnsx-horae-tests-0.1.0` and
`4ybli4yj1nq6m0zxb1pwsz268bqxp5zx-horae-sqlx-prepare-0.1.0` (all `.drv`).
No passing executable result for #258 is claimed yet. #255–#257 descriptions
now record their completed full gates while retaining draft/retargeting limits.

Next collect `64524` and run #258's full native gate, then verify composition
with #241 and the other reader/editor deliveries before continuing connected
UI extraction. Preserve the explicit approval-editing limitation. Original
hunk accounting remains incomplete. No extraction PR was merged or closed.

### Combined Timesheet and permission verification started

Previous turn made progress: published #258 and completed the three standalone
reader gates. Reconfirmed #216 merged at `02f7b58` before changes. The original
#212 worktree still has the same18 unpublished paths; none was edited.

Created verification-only branch `integration/timesheet-permission-check` in
`.worktrees/timesheet-permission-integration`, starting at #258 `d2b45ca`.
Composed prior reader/editor verification `7a2d61c` locally as `faec57b`, retaining
both sides of three registration conflicts: models, HTTP harness and profile
test modules. Then composed #241 `7820f8d` without conflict as
`6da9981ebe9680b5c0d66bc0e408f09d060d56a7`. This branch is published for
reproduction only; no delivery PR, GitHub merge or activation occurred.

The combination contains #240/#241/#250/#253–#258 and their inherited
foundations. All three source heads are ancestors. Twenty-one selected complete
reader/editor/command/legacy-writer source and test files match their owning
heads exactly; registrations keep every suite. Formatting `90772` passed with
zero changes and the worktree is clean. Nix tests/Clippy/live SQLx `25306` is
running on exact `6da9981`, currently in Clippy; full native gate remains pending.
This composition evidence does not certify the remaining original UI or task
lifecycle groups.

#258 `64524` remains live on unchanged `d2b45ca`: strict Clippy and live SQLx
passed, core compilation finished and the application test build is in progress.
Do not restart it merely because a poll is quiet. Its full native gate has not
yet been launched.

Read-only dependency audit clarified the next boundaries. `b8b1c60` changes
the existing `submit_week` signature to require `TimesheetWriteContext` and
updates its actual page caller. Extract it with the connected Timesheet UI,
its124-line submission tests and97-line session HTTP tests; do not ship an
incompatible standalone endpoint or invent an alternate compatibility API.
The original still restricts this submission route to legacy-own context.

The command implementation at final original `db3935d` differs from #258's
`02c4245` source by exactly the later `0591407` task-activity addition: one
`AND ($5 OR pt.active)` query condition and49 lines of archival test. These
belong with migration0048 and the project-task lifecycle extraction, alongside
the matching changed SQLx descriptor. Keep that remaining ownership explicit;
#258 does not claim the final task-archival behavior. The other command files
and DTO have no later differences in that comparison.

Next collect `64524` and `25306`, launch their full native gates after targeted
checks finish, and retain exact-head evidence. Then extract the coherent
Timesheet UI/weekly-submission group from `e1ddd9a`, `a0632a8`, `b8b1c60` and
later focus/person-switch/date-offset fixes, preserving browser coverage and
keeping task lifecycle changes separate. Original hunk accounting, editor,
report, project/task UI and unpublished client groups remain unfinished.

The connected UI extraction must also preserve current master navigation work:
its shared navigation script already handles client drafts, while original
`a0632a8` adds Timesheet on top of permission-editor guards not yet extracted.
Apply the owned Timesheet condition without overwriting client behavior; preserve
the permission-editor additions for their owner. The original editor-navigation
test file is absent from #258's base and needs explicit test ownership/wiring,
not a blind whole-file replacement. Timesheet page and sidebar themselves match
the pre-consumer source base; route/admin-shell files have other changes to keep.

### Cancellation-test failure preserved and synchronization corrected

#258 old-head `64524` terminated exit1 on `d2b45ca`:1010 application tests
passed, one failed,11 ignored. All13 new command tests passed. The failure was
`sheet_holds_selected_activity_through_entry_delivery_and_releases_on_cancel`,
at the five-second archive completion timeout (`timesheet_context.rs:343`),
not an authorization assertion. Strict Clippy and live SQLx passed; the overall
gate failed. No unchanged rerun was used to discard this failure.

Inspected the pinned SQLx0.8.6 source in the actual Nix vendor tree:
`sqlx-core/src/transaction.rs` Drop calls `start_rollback`;
`sqlx-postgres/src/transaction.rs` queues that rollback behind the current query;
pool return pings/flushes it asynchronously. Aborting the Rust task does not
immediately cancel the blocked PostgreSQL statement. The test retained its
artificial ACCESS EXCLUSIVE table barrier after confirmed task cancellation,
making queued cleanup compete with both the5000ms statement limit and the
five-second archive deadline. No production-code change was needed.

Owner #257 commit `b30e3cda8ca6f076294735eb9c8e9c03b17bd56e` adds four lines:
after confirmed cancellation, release that artificial barrier so SQLx rollback
can complete. Every original assertion remains, including the observed lock
dependency before cancellation, cancelled-task result, bounded archive
completion and subsequent access denial. No timeout was increased, test
removed/ignored or grant/runtime behavior changed. Formatting `7799` passed
with zero changes. This is an explicit test-harness adaptation, not a claim
that the database query itself now cancels immediately.

Propagated the owner fix without rewriting history: #258 is now
`b0eacfdcc08f2d3e226cf3a3e16a9fb25913cef8`; verification-only combination is
`015dcd15f97c299fef0799041849f9ffbf8696c0`. All three heads are published.
The old #257 passes at `8e09e60` remain historical, not current-head evidence.
Descriptions for #257/#258 now expose the failure, correction and pending gates.
Old combined `25306` remains live on `6da9981` in application tests; do not
mistake its result for verification of `015dcd1` or restart it while live.

Fresh #258 tests/Clippy/live SQLx `98625` runs on `b0eacfd`. Full native Nix
#257 `40402` runs on `b30e3cd`. After collecting old combined `25306`, start
fresh combined checks on `015dcd1`; #258 still also needs its full native gate.
No corrected-head passing executable result is claimed yet. Continue the
coherent UI extraction only with these failures/limits preserved in the ledger;
the goal is still incomplete and no PR was merged or closed.

### Selected-person Timesheet UI extracted; corrected command tests pass

Reconfirmed the user-priority optimization PRs #251/#252 are merged at
`35dc414` and `ed558f6`; extraction merge authority has not changed.
Old combined `25306` terminated successfully on `6da9981`:1037 application
tests passed, zero failed,11 inherited ignored. Current corrected combination
`015dcd1` has its own full native Nix gate `64407`, still running. Do not
transfer the historical pass to that head. Corrected #258 `b0eacfd` targeted
tests/Clippy/live SQLx `98625` now passed, including the previously failing
cancellation test:1011 application tests passed, zero failed,11 ignored.
Its remaining full native gate is `7633`. #257 `40402` remains running.

Created isolated `feat/timesheet-selected-person-ui` at #258 `b0eacfd`,
then published unsigned `0dfea8b791aebb39056d308be1c92db7fa1e25fa` as
draft #259. Git reports29 paths,2441 additions and381 deletions (30 physical
paths when the cache rename is counted separately). This coherent group
owns the original UI and the inseparable weekly-submission caller/contract.
No CSS, migration, policy activation, original worktree or real data changed.

Nine complete files match final original `db3935d`: Timesheet page, four
helper/test files, scoped-time DTO, own-submission HTTP fixture and both
Timesheet browser suites. The original submission patch and124-line race
tests are carried without changing assertions. Five original SQLx descriptors
replace the obsolete organization-config query descriptor; the removed
version remains recoverable in Git. Other source is deliberately not copied
wholesale: route/admin-shell retain current client/audit behavior, navigation
retains Clients/Projects/Invoices guards, modal fixture retains current
project-list endpoint, and New Project keeps its newer client/focus assertions.

The common navigation harness is extracted with all assertion bodies retained:
its permission-editor scenario is reserved for that future UI owner; this
branch tests the already-present client scenario instead, plus invoice,
project and Timesheet. Browser runner retains every current suite and adds
navigation, modals, Timesheet errors and selected-person permissions only.
Readiness changes touch only the Timesheet endpoint in shared layout/menu/
style fixtures. Canonical fixture activation occurs only in the runner's
disposable database, never an existing instance.

Bounded adversarial source review covered complete-page identity checks,
no partial totals, selected-person remount/history, dirty/pending navigation,
command context and owner-only recovery, lock states, submission policy/
activity fencing, SQLx ownership and unrelated shared-screen preservation.
The unchanged original still does not implement canonical submitted editing
or weekly submission, and task-archival condition `0591407` stays with its
lifecycle migration. Existing mouse-only calendar interactions, placeholder
notes labeling and inherited icon conventions are not certified as accessible
or redesigned in this split.

Rust/testing/async guidance and Impeccable's audit applied; the latter prompted
explicit shared-surface and inherited-accessibility limitations, not design
changes. Static UI detector returned an empty finding list; no visual pass is
claimed from that. Navigation script `65546` passed all10 tests; formatter
`44182` passed with zero changes and whitespace/provenance checks passed.
Full native gate `48062` runs on exact `0dfea8b`. Later combined verification
must include this head; `64407` does not yet contain #259.

Next collect the live standalone and combined gates without restarting them,
fix only demonstrated extraction/compatibility faults, then compose #259 with
the other extracted permission work and verify that new head. Continue
remaining permission-editor/People UI, reports, project/task lifecycle/UI,
unpublished Clients and original-hunk accounting. No extraction PR was merged
or closed, and the goal is not complete.

#258 full native Nix `7633` subsequently terminated exit0 with all checks
passed on exact `b0eacfd`. It reused available derivations and rebuilt the
format check; do not report a fresh execution of every cached test. The
targeted executable pass `98625` above remains its new-head test evidence.
PR258's description now records both. #257 `40402`, combined `64407`
and UI #259 `48062` remain live; #259's WASM client build has passed,
which is not its complete release/server/browser gate.

Read-only preparation for the next independent UI group found that final
`admin.rs` also contains later task-catalog mutations (`TaskRateEdit` and
the requester-bound `create_task` signature). Those belong to the unfinished
task owner, not a blind People/editor copy. `ee16165` ties CanonicalPeople,
requester-bound dialog selection and shell/sidebar access together; the
existing reader/editor combination `7a2d61c` already contains #250/#253.
Review the editor/recovery/template files fully before extracting that
consumer, preserve legacy task behavior, and reconcile its shared navigation
guard with #259 in a later integration check. No editor UI edits made yet.

### People/editor extraction and two Timesheet dependency corrections

Published draft #260 at `98857cfa0defe009447c4cb02cce4d4a54aa2d7a`,
based on already-verified `7a2d61c`. The23-path consumer group contains
4809 additions and126 deletions; its bulk includes the1988-line Rust UI
fixture and570-line real-browser recovery fixture, neither shortened. Fifteen
whole files match final original `db3935d` by Git blob hash: all eight
People/editor modules, both DTOs, storage JavaScript, Rust UI fixture, two
browser recovery/storage fixtures and browser.nix.

Historical `ee16165` supplies admin/shell/sidebar wiring without later
task-catalog changes. Timesheet route-user props are excluded from this
independent base, not removed from #259; later composition must retain them.
Only permission-editor endpoint exports are exposed here. Shared navigation
retains current Clients and adds permissions; its tests retain original
assertions with the independent branch's client scenario in place of the
not-yet-present Timesheet scenario. Combined verification must cover all five.
No CSS, new dependencies, migration, activation or real data changes in #260.

Bounded source review covered directory paging/requester identity, stale
response rejection, preview/confirmation, tab/session/org recovery binding,
storage before mutation, exact retries, self-demotion acknowledgement cleanup,
template limits and loss-of-authority recovery. Legacy People/tasks remain
when policy is inactive; canonical UI never substitutes for server checks.
JavaScript gate `31361` passed16 tests; formatter `27533` passed with zero
changes before the restored original Nix hunk; whitespace/provenance passed.
Impeccable detector returned no findings, not visual certification. Full native
gate `66677` runs on exact `98857cf`; no executable Rust/browser pass yet.

#259 initial full gate `48062` failed at `0dfea8b` before browser scenarios:
only tests/browser was copied into the Nix store, while navigation tests load
application JavaScript by relative path. Release server/WASM and strict Clippy
passed, not the overall gate. Commit `455c155` restores the exact original
`4294aa3` browser.nix hunk, copying the application subtree so assets remain
beside tests. #260 independently carries the same shared prerequisite.

Corrected-head gate `91457` then passed navigation10/10, modal, error and
initial selected-person scenarios, but failed the original calendar assertion:
moving an entry one hour stored start255 instead of240. Inspection identified
the omitted original `5f7895c` two-line CSS dependency: nested event text was
the mouse-coordinate target instead of the event box. Commit
`4ce091923c1ce89fcd478a50a5ea96c1358f8189` restores exactly that hunk.
Selectors occur only in Timesheet; current Clients styles and resize handle
remain untouched. No assertion or expected minute changed. Fresh full gate
`75088` runs on that head. Both failures remain evidence, not discarded reruns.

#257 current-head full native Nix `40402` terminated exit0 with all checks
passed at `b30e3cd`:998 application tests, zero failed,11 inherited ignored,
plus remaining suites, browser and NixOS/OIDC. Combined `64407` at `015dcd1`
has passed1037 application tests with zero failed/11 ignored and advanced to
deployment gates, but is not yet a complete pass. It excludes both UI PRs.

Next collect `75088`, `66677` and `64407` without duplicate runs. Fix only
demonstrated extraction faults. After standalone verification, compose #259
and #260 with the wider combination, preserving all navigation scenarios,
Timesheet user route props and every existing browser suite; verify that new
head. Continue remaining report/project/task/Clients groups and complete
original-hunk accounting. Optimization #251/#252 remain the only authorized
merged priority deliveries; no extraction was merged or closed.

### Report readers extracted; earlier combination verified

Previous goal turn made progress: #260 and original #259 dependency corrections
published. Current combined gate64407 terminated exit0, all checks passed,
on015dcd15f97c299fef0799041849f9ffbf8696c0:1037 application tests, remaining
binaries, browser and deployment/OIDC. It excludes the later UI/report PRs.
Original #212 retains the same18 dirty paths; #216 reconfirmed merged before
edits. No originals, #208, real data or activation state changed.

Draft #261 bf452ddc260ac5d1a2427b8f115d767eae6500b2 owns75f13a1/a23804f
detailed reporting plus final detailed active-project/billability refinements.
Base #257 supplies unchanged admission; only original helper visibility changes.
Final reader differs only by deferring grouped registration. All733 lines/
13 DB tests match final original;122-line HTTP fixture preserves75f13a1 reader
assertions and exact a23804f totals/pagination additions. Export/access/group
assertions remain with their owners. DTO runtime matches detailed original;
historical pre-consumer web expectations remain until UI wiring.

Static inventory matched38 macros to35 original descriptors;19 absent
descriptors copied exactly. Format45906 and whitespace passed unchanged.
Owned diff29 paths/1512 additions/one deletion. Full native Nix37229 runs
onbf452dd; live schema/executable acceptance remains pending.

Draft #26266dbf0bcefd1dee781e9daf9683b40d6235533b3 stacks on #261.
Final grouped reader153 lines and DB fixture722 lines/13 tests matchdb3935d;
171-line HTTP reader fixture matches41ff137. Endpoint33 lines original,
group DTO web expectations retained until consumer wiring. All32 macros
match27 original descriptors;10 absent descriptors restored. Format73230 and
whitespace passed unchanged. Owned diff18 paths/1385 additions. Full native
Nix79456 runs on66dbf0b; no executable pass claimed.

Bounded report review covered scope before filters/aggregation, qualified
historical parents, distinct entity IDs, requester/policy denial, active actor/
grant locks, revocation/cancellation, strict cursors/enums, exact64-bit totals,
effective/frozen rounding and billability, empty/exhausted pages and one-statement
snapshots. No new dependency/schema/CSS or legacy endpoint replacement.
No high/critical finding in the inspected boundary.

Scoped Spec Kit analysis used #248's spec/plan/tasks and original report
contract. Initial prerequisite invocation resolved stale metadata to016 and
failed for its absent plan. Inspected resolver, then explicitly selected015
with SPECIFY_FEATURE_DIRECTORY in paths-only mode and independently checked
all three required files. No feature.json persistence, extension hooks or spec
edits. This is a read-only invocation adaptation, not an initial-command pass.

FR-006/007/008/010/018 and report-relevant SC-002/003/006 map to T201–T202,
T213–T215, T219–T220 and reader portions of T222/T235: eight scoped requirements,
nine relevant tasks, all mapped. No new ambiguity, duplication or constitutional
conflict found in this boundary. Mapping is not implementation completion.
T203 remains open; full candidates, exports, consumer, financial families and
governance/activation remain separate. No spec remediation required/performed.

#25975088 has passed release/Clippy and the exact Calendar create/move/resize/
reorder/delete assertions after restoring original CSS. #26066677 has passed
release/Clippy and permission recovery/history/storage browser scenarios.
Their complete browser and remaining gates are still running; no full passes.

Next collect75088,66677,37229,79456 without duplicate runs. Compose #259/#260
with verified015dcd1 after standalone results, preserving all five navigation
guards/tests and route props, then verify that head. Compose report readers
and extract matching exports/access/consumer next. Original4294aa3's New Project
dev-login isolation hunk stays with project-editor fixture ownership alongside
later permission-label changes. Reports, project/task lifecycle/UI, unpublished
Clients and complete hunk reconciliation remain open. No extraction merge.

### Combined UI and report-reader verification

Reconfirmed #216 merged before changes. Priority #251/#252 are merged at
35dc414/ed558f6; no extraction PR was merged or closed. Original #212 remains
db3935d with the same18 dirty paths, preserved without edits.

Local verification branch `integration/timesheet-permission-check` now contains
#2594ce0919, #26098857cf and #261/#26266dbf0b on verified015dcd1. Integration
commits89d6377,be9e898,1a879615f35b285d29f3e7e9d60abc84e09c6fbc preserve
both parents; no original branch was rebased or rewritten. Composition proceeded
while remaining standalone gates ran, without treating those gates as passed.

Three UI conflicts were resolved by retaining all five navigation kinds and
both original scroll-coordinate scenarios. Tests now exercise13 navigation
cases plus6 recovery-storage cases: pinned Nix run17002 passed19/19. The
subsequent indentation-only adjustment is included in the fresh full gate.
The runner retains the union of all29 suites, no duplicate or missing names;
permission recovery remains before database-mutating legacy browser fixtures.
Timesheet `user` route props survive in shell, sidebar and shell-test stubs.
The report HTTP registration conflict retains every check call from both
parents; no assertion body was removed or weakened.

Owned Timesheet modules/DTO/browser fixtures, People editor modules/browser/
component tests, and report reader/DTO/DB/HTTP fixtures match their respective
extraction heads exactly. Only shared integration points changed. Format31039
passed with zero changes; whitespace passed. The targeted Impeccable shell
detector returned exit0 without diagnostics, not a visual or accessibility
certification. Full native Nix88592 runs on exact1a879615; not yet a pass.

Standalone #26066677 terminated exit0, all checks passed on98857cf: release,
strict core/server Clippy, complete browser, SQLx,1011 application tests (zero
failed,11 inherited ignored), remaining component suites and deployment/OIDC.
#25975088 passed release/Clippy/browser/SQLx,1032 application tests (zero failed,
11 ignored) and remaining suites; deployment checks now run. #26137229 passed
release/Clippy/browser/SQLx and starts application tests. #26279456 passed
release/Clippy and continues browser checks. Only #260 is a complete new pass.

Next collect75088,37229,79456 and88592 without duplicate builds; record
terminal results against these exact heads. Continue the original report export/
access/consumer extraction, then project/task and unpublished Clients ownership,
and finish hunk reconciliation. The combined branch is only a verification
artifact, not a proposed broad delivery PR or authorization to activate policy.

### Scoped spreadsheet extraction published

Previous turn was progress: published UI/report integration1a87961 and recorded
#260's completed gate. This turn #25975088 terminated exit0, all checks passed
on4ce0919 (1032 application tests, zero failed,11 inherited ignored, remaining
suites, browser and deployment/OIDC). #26137229 also terminated exit0 with all
checks passed onbf452dd. No repeated build or extraction merge.

Created prerequisite worktree/branch `integration/scoped-time-export-prerequisites`
at e8b95cc5365817d4998492b913ec6f5cf322d3e3 by composing #261 with #247.
One HTTP registration conflict retained every original module and check call.
This base has no independent acceptance claim; its combination is exercised by
the dependent full gate. It is not a delivery PR or merge target.

Draft #263 at8cc11c30757e177846530a0544f6808e337d792c owns cbc78a8's bounded
XLSX materialization and captured-scope release authorization. Original613-line/
11-test DB fixture is byte-identical. Original72-line HTTP addition coexists with
#261's totals/pagination assertions. Snapshot test adaptation exactly matches
cbc78a8: either a coherent earlier snapshot or a later size rejection is valid;
the size guarantee and subsequent rejection assertion remain enforced.

The231-line export reader differs from historical cbc78a8 only by the final
original active-project/billability SQL predicates and bindings. Its query DTO
already contains these fields; the legacy URL adapter initializes their existing
defaults. Full shared URL/requester/policy parsing remains with CSV/consumer work.
No UI, dependency, migration, real-data mutation or canonical-policy activation.

Review covered tenant/session binding, fail-closed policy/catalog facts, scope
before limits, exact minutes, private-field exclusion, source identity capture,
rendering without authority locks, fresh post-render revocation, empty exports,
actor commit/rollback races and cancellation. No new high/critical boundary
finding. SQL inventory:38 macros/26 unique queries;11 absent cache descriptors
copied byte-for-byte fromdb3935d. One obsolete original size-query descriptor
removed after checking no remaining Rust source contains its query; recoverable
from Git. Owned diff20 paths/1269 additions/103 deletions.

Formatter37857 changed only the combined HTTP module ordering. Initial source
commitcf7c07d omitted that tracked formatting delta; follow-up8cc11c3 includes
it. Active gate48482 evaluated the formatted worktree before the follow-up commit.
Clean-head evaluation67972 proved all ten native check derivations identical to
the active gate, including package, tests, browser, SQLx, Clippy, formatting and
both deployment checks. This is input-identity evidence, not a completed pass;
do not restart an identical build. Both branches and draft are published.

Next collect79456,88592 and48482. Continue09bd15f CSV delivery on the proper
#249 prerequisite, then shared filters, grouped exports and report consumer;
retain all deferred original assertions. Projects/tasks, unpublished Clients and
complete hunk accounting remain required. #212/#217/#208 remain untouched.

### Scoped CSV and shared download filters extracted

Previous turn progressed by publishing #263. This turn #262 gate79456 exited0
with all compatible native checks passed on66dbf0b:1024 application tests,
zero failures,11 inherited ignored, remaining suites, browser, SQLx, Clippy,
format and deployment/OIDC. Its PR body now records the completed evidence.
Wider combination88592 passed browser and live SQLx and is running application
tests. XLSX48482 passed release/Clippy and is running browser checks.

Created prerequisite branch/worktree `integration/scoped-time-csv-prerequisites`
at4e0ed43368fd428538b1f8ee3dc84e6c1c8020c1 from #2638cc11c3 and #24971232dc.
The sole limits-module conflict retained both time and project registrations;
automerged HTTP registry retains CSV routes and all reader/XLSX calls. No
independent base-gate claim. It is a review base, not a delivery merge target.

Draft #26455d362b8aa811a02a9e1f944e2625ab6b52fff0e owns09bd15f and93aaa68's
CSV delivery and shared URL filters. Source adaptation retains the final193-line
native cursor, including existing active-project/billability DTO predicates.
The URL adapter keeps their legacy false/Any defaults; later URL controls,
expected-policy binding, grouped exports and consumer tests retain other owners.
Existing #222 native stored-row decoding is reused, not duplicated.

Exact-source comparisons passed for the native cursor,222-line delivery module,
623-line/10-test DB fixture,149-line/five-test parser fixture and238-line
CSV/XLSX HTTP filter fixture. Historical HTTP context was adapted only to retain
#261 full-period totals; no existing pagination, XLSX or legacy assertion removed.
Shared XLSX authorization helpers match the original09bd15f refactor.

Review covered captured DECLARE authority versus current release authority,
strict native-state validation before empty sentinels, tenant-qualified parents,
source reassignment/deletion, exact effective minutes and billability, metadata
byte limits, pending-context revocation and lock-free capacity waits. Five
filter dimensions narrow rather than grant authority; malformed/ambiguous keys,
download cursors, incomplete bindings and switched identities remain denied.
No new high/critical boundary finding from source review; executable evidence
for this head remains pending. No UI/schema/dependency/real-data/activation change.

Scoped SQL inventory found47 macros/31 unique queries, with every macro parsed
and matched to its descriptor.17 absent descriptors were restored byte-for-byte
fromdb3935d; two obsolete native-cursor descriptors were removed only after
confirming their SQL no longer appears in remaining Rust sources, recoverable
from Git. An initial whole-tree heuristic did not handle concatenated macro
queries; it was not treated as authoritative or used for unrelated deletions.
The scoped inventory is complete; live SQLx remains the executable gate.

Formatter31607 and whitespace checks passed without edits after staging all
new files. Commit55d362b is clean and both branches are published. Full native
Nix gate40286 is running on that exact head; do not claim completion or restart
without changed evidence. Existing tasks T207–T212 trace this extraction;
T203, grouped delivery, pickers and financial reporting remain incomplete.

Next collect88592,48482 and40286 without duplicate builds. Extract grouped
XLSX/CSV and their preserved HTTP assertions, then the Reports consumer on the
actual reader/export foundations. Continue project/task/Clients extraction and
complete original-hunk ownership. No extraction merge or closure occurred.

### Report access prerequisite and grouped downloads published

Previous turn was progress: #264 and its conservation record were published.
This turn reconfirmed #216 merged at02f7b58 before edits and polled the same live
gates rather than restarting them. Full composition88592 on1a87961 has now
exited0 with all compatible native checks passed:1098 application tests,
zero failed,11 inherited ignored, remaining suites, browser, SQLx, Clippy,
format and deployment/OIDC. This combines Timesheet/People/readers but excludes
later exports and the future Reports UI; do not expand that acceptance claim.

Grouped source inspection exposed a real prerequisite: its strict transport
requires TimeReportPolicy and expected-policy download binding from7266abb.
Extracted this backend boundary first as draft #265 at
f436a29bdd5bfaacdd05a6b00b6f9e680699e9f3, based on #26455d362b.
Original access endpoint/helper/DTOs, source-policy admission and parser/HTTP
tests were preserved. The complete parser and registered-session fixtures
match7266abb exactly; query/page web lint expectations remain until their
actual consumer exists. Nine files,256 additions/eight deletions; no SQL change.
All12 inspected macros map to11 existing descriptors. Format45050 passed with
zero changes; full native Nix1825 is running on the clean published head.

Access review checked session-derived identity, optional requester mismatch,
fresh policy and grants, fail-closed corrupt/missing authority, information-safe
errors and stale links in both policy directions. Both formats reject mismatched
mode before source selection; inherited release checks reject later mode changes.
This is T216–T218's backend prerequisite, not UI/browser completion or T203.

Composed #265 with #262 in isolated prerequisite worktree/branch
integration/grouped-time-export-prerequisites atda493f6. The one HTTP-fixture
conflict retained all access/mode/export-filter assertions and the grouped
reader call; module registration was organized without dropping either side.
This is a review base with no independent gate claim, not a delivery merge target.

Draft #266 at0eec1a478e2d04068a923fb5a36d181266111638 extracts ca170c0/2b59b58's
grouped CSV/XLSX transports on that base. Eight whole-file comparisons passed:
handlers/renderer, final original XLSX/native cursor SQL, seven XLSX DB tests,
nine CSV DB tests, streaming coordinator, shared delivery authority and the
41ff137 multi-filter HTTP fixture. Both original production routes and their
HTTP harness registration are retained. Source SQL includes final original
active-project/billability predicates to match the inherited query DTO; the
later URL controls, extra fixtures and UI/browser links retain separate owners.

Grouped review checked distinct IDs despite identical labels, scope before
aggregation, exact integer totals, one-snapshot workbook payload/context bounds,
captured pairs after source edits/deletion, canonical-only admission and empty
sentinels. Streaming checks at most128 pairs at once under one group-wide
authority lifetime, reserves output before gates, and sends only after the last
context and successful gate release. Native metadata/Unicode byte weighting,
cancellation and more than10,000 groups/entries retain their original tests.
Reviewed against grouped contracts in csv-exports.md and time-reports.md.
No new high/critical source-boundary finding; runtime verification is pending.
No policy activation, schema/dependency/UI change or real-data mutation.

SQL inventory34 macros/28 unique queries;18 absent descriptors restored exactly
fromdb3935d. Formatter72873 passed with zero changes; whitespace/source checks
passed. Published owned diff34 files/2039 additions/13 deletions. Full native
Nix40587 runs on clean0eec1a4 and includes both prerequisite branches. It does
not certify consumer/browser integration or later filter transport. T221 remains
partially owned by the future UI; T203 remains open.

#26440286 passed its release build and entered Clippy. #26348482 passed release,
Clippy/browser/live SQLx and is running application tests. #2651825 and #26640587
are building their release applications. Original #212 still has the same18
unpublished paths; #212/#217/#208 were not modified or merged.

Next collect48482,40286,1825 and40587. Extract the remaining shared
active-project/billability URL controls and their original tests, then the Reports
consumer on the reader/export/access foundations. Fold these into the wider
composition only after preserving its existing fixtures. Continue project/task
and unpublished Clients boundaries and complete original-hunk accounting.

### Shared result-filter transport and fixtures published

Previous turn progressed by publishing #265/#266 and recording the successful
Timesheet/People/reader composition. This turn reconfirmed #216 merged and the
four pending gates live. #26348482 subsequently exited0 with all compatible native
checks passed on8cc11c3:1064 application tests, zero failed,11 inherited ignored,
remaining suites, browser, SQLx, Clippy, formatting and deployment/OIDC.
The previously recorded ten-derivation identity proof binds this run to that
clean published head; no identical build was restarted.

Created isolated worktree/branch feat/time-report-download-filters on #266.
Draft #267 at e029a892e228a076887d594750d48fff2fa60b9d owns the remaining
de8f9ad/db3935d URL-filter transport: optional strict active_projects_only and
the closed all/billable/non_billable selection, forwarded into already extracted
query fields and SQL. No production SQL, UI, CSS, schema or dependency change.
No activation or real-data mutation.

Preserved the original347-line/four-test active-project fixture and378-line/
three-test billability fixture byte-for-byte. They exercise detailed and grouped
CSV/XLSX, all four dimensions, empty-result authorization, archived client/task
independence, effective project/task billability, invoice-linked frozen zero
rounding, private-field exclusion and source snapshots after edits/archiving.
The final full HTTP filter fixture and grouped handler/parser file are exact;
the detailed parser fixture differs only by the two-line Project-export test
registration retained for that separate owner. No existing test weakened.

Source review checked strict default/duplicate/invalid query decoding, including
flattened grouped URLs, and traced the forwarded fields through each export's
existing predicates before aggregation and size limits. T222/T235 contracts
apply to this transport boundary; T236/UI resource keys and browser interactions
remain with the Reports consumer. No new high/critical source finding; executable
evidence for this head remains pending.

SQL inventory29 macros/26 unique queries;10 absent test descriptors restored
exactly fromdb3935d. Formatter23013 and whitespace checks passed with no edits.
Owned diff18 paths/1121 additions/two deletions, predominantly original fixtures.
Branch and draft are published; clean-head full native Nix51012 is running.

#26440286 has passed release/Clippy and is exercising browser checks;
#2651825 passed release and is running Clippy; #26640587 is building release.
Next collect these handles and51012 without duplicates. Extract the full
original Reports consumer, including its component/browser fixtures and filter/
group/expanded request keys, on the now available reader/export/access/filter
foundations. Then verify wider composition and continue project/task/Clients
ownership and the complete original-hunk audit. No extraction merge or closure.

### Reports consumer and wider composition published

Draft #268810ce57 preserves the remaining ordinary Reports consumer on #267:
three scoped modules, authenticated mode-gated route,15 original component
tests and the538-line disposable Chromium fixture. Eight paths,2566 additions/
43 deletions. No shared CSS, SQL, cache, schema, dependencies or activation.
The three modules, browser fixture and DTO are byte-identical todb3935d.
Route and test stub retain the existing no-argument project-tag API; only that
future Project-reader signature is deferred. All test assertions are preserved.
This assigns the remaining Reports UI/browser hunks from7266abb,41ff137,
e2e66fb,ca170c0,2b59b58,ecac66b,de8f9ad anddb3935d to #268; each already
extracted backend/specification owner remains unchanged.

Source review traced access-before-mount, pinned requester/mode, exact ready
resource keys including every filter/context, stale-response exclusion,
integer totals, escaped labels and full-period bound downloads without cursors.
Existing design contract explicitly requires incumbent shared components and
no shared-CSS changes, not copying the report-builder prototype. Keyboard,
responsive and theme assertions are preserved, not claimed executed yet.
T203 full candidate discovery and financial-family requirements remain open.
Formatter63610 passed without edits; whitespace and browser syntax52651 passed.
Clean-head full native gate50800 is running; the draft states pending acceptance.

Created isolated integration/reports-permission-check atb7836d7, combining
previously verified1a87961 with #268 and its complete export foundations.
Resolved only HTTP module and browser runner registration conflicts by retaining
both sides; all30 unique browser suites remain, with permission recovery and
Reports before legacy fixtures. Dedicated Reports and Timesheet/People source
and tests match their parents exactly. Formatter3606 and shell syntax passed.
Full native25958 is running; no identical check was restarted. This branch is a
verification artifact, not another delivery PR or authorization to merge.

Next collect40286/1825/40587/51012/50800/25958 without duplicate builds and
record actual outcomes. Continue Project reads/editor, task lifecycle/consumers,
unpublished Clients preservation and complete hunk accounting; the overall
separation goal is not complete. Ledger state through #267 was published
at18489b3 before this iteration. No extraction PR merged or closed.

### Conservation refresh and Project extraction boundaries

Previous iteration was progress: #268 and the wider composition were published.
Reconfirmed #216 merged at02f7b58; #212db3935d and #217dd141c5 remain open/draft,
and #20848a4156 remains open and untouched. Original tracked edits still compare
identically to preserved snapshotd364270; tar comparison of the six original
untracked files against the private backup also passes.

Read-only conservation audit used the live heads of46 open extraction PRs
(#219–#268 excluding #229/#230 and the merged build-only #251/#252), plus
mastered558f6. Compared Git blob IDs against the raw, no-rename9301112..db3935d
change set:373 non-SQLx paths and867 SQLx paths, including46 removed descriptors.
Exact final blobs occur in at least one candidate for160 ordinary/tooling files,
48 specs files and640 surviving SQLx descriptors. Compared with the earlier
35-PR audit this adds54 ordinary and120 descriptor matches. The158 ordinary,
seven specs and181 surviving descriptors without a whole-file match still need
hunk/equivalence ownership; they are not proven lost or absent. These counts
are conservation evidence, not test completion or a completion percentage.
Historical cache removals and integration-only branches are not credited as
delivered exact blobs by this audit.

Project boundary inspection traced2497dbe and2631186 against both the original
final tree and current extracted code, including the full project-read contract:

- Editor2497dbe: preserve its catalog/protected-field/save transaction, bound
  form and original tests as an editor responsibility. Existing #254 already
  carries the delegation/writer foundations; rate evaluator and project
  management modules compare exactly to the original final blobs even where
  commit ancestry differs. Do not duplicate those modules or infer a missing
  dependency solely from ancestry.
- Reader2631186: keep overview/details/tags/team/spend and configured-budget
  projections coherent with optional money and the summary/breakdown DTOs.
  Budget summary must be computed before private-detail filtering, and the
  trusted alert calculation must retain its unfiltered service semantics.
- Reader delivery: project CSV/XLSX and compatibility list/count/direct-ID
  consume the same row/field rules but retain their own bounded release checks.
  Consumer signature changes must include every caller and component stub,
  including the two Project-tag adaptations intentionally deferred in #268.
- Later task activity/canonical catalog work remains separate. The original
  Project-read fixture at2631186 does not yet register the later406-line task
  fixture; its final parent changes only that registration and helper visibility.
  Preserve these later hunks for their task owner rather than silently pulling
  task lifecycle into the project-reader extraction.

Concrete preservation hazard: replacing project_creation.rs wholesale with
db3935d would revert master's shared client-profile/default-rate validation to
the older inline checks. The2497dbe editor patch does not change that function;
extract its owned hunks and retain master's client validation. No runtime edit
was made during this inspection.

All six existing full-gate handles remain live. #26440286 has now passed
1095 application tests, zero failed/11 inherited ignored, and the remaining
application suites; deployment checks are still running. #2651825 and #26640587
are building/running tests; #26751012 passed release/Clippy and is in browser;
#26850800 and composition25958 are building. No restarted or duplicate builds.
Next extract the existing Project editor on the actual shared foundations,
collect these terminal outcomes, then finish Project reads/delivery, tasks,
unpublished Clients classification and complete hunk-level accounting.

During final collection40286 completed successfully on #26455d362b:
all compatible native checks, including browser, SQLx, Clippy, formatting,
server/WASM and deployment/OIDC. No pending local gate remains for that head;
retargeted remote acceptance and wider final composition are still separate.

#26850800 terminated after successful production server/WASM build: Clippy
found CurrentUser/UserListItem unavailable in its standalone component fixture.
Those DTOs belong to the identity/directory extractions, not Reports runtime.
The existing branch APIs both return User. Commit aef180f changes only those
two test-stub type references to the actual base API; all15 tests/assertions,
production modules and browser fixture are unchanged. Formatter26328 passed,
fix pushed, clean-head full64831 launched. Do not credit50800 as a pass.
The wider b7836d7 composition has the projected DTOs and continues under25958;
its old test fixture is not silently claimed identical to the corrected #268.
Reconcile this test-only difference in the next wider composition, preserving
the actual API types and every assertion. No duplicate live check restarted.

### Project editor extraction started; Reports access gate complete

Confirmed #216 remains merged at02f7b58. Created isolated
feat/scoped-project-editor from the fully verified shared-reader/editor
composition7a2d61c. Its editor source boundary is2497dbe, not the later final
tree containing task lifecycle changes. Preserve current client validation and
reuse the already-extracted rate/delegation implementations.

Full native1825 completed on #265f436a29 with exit0 and all compatible checks
passed, including deployment/OIDC. Updated its PR verification; no extraction
merge or closure. Other architectures were not executed. The current265 head
was reconfirmed before updating its body.

Next finish the Project editor extraction and its original regressions, collect
40587/51012/25958/64831, and continue the outstanding reader/task and hunk
accounting work. No check is restarted while its existing handle is live.

Published Project editor extraction #269 atd5dc1da on
feat/scoped-project-editor, based on7a2d61c. Its82 changed paths comprise
34 source/test/runner paths and48 original SQLx descriptors. The isolated
responsibility is bound editing, protected-field intent and atomic manager
selection; no Task lifecycle or policy activation is included. Original
2497dbe database/HTTP/component/browser regressions are retained, including
the4294aa3 single-DEV_LOGIN-administrator fixture correction. The scoped query
inventory parsed282/282 macros and restored only missing descriptors after
query/hash equality checks. No descriptor was deleted by a global heuristic.

Two master-preservation adaptations are explicit: retain the shared client
validation in project_creation.rs and the NewProjectForClient/client_context
flow (saved drafts retain precedence) in pages/new_project.rs. HTTP module and
browser registrations retain all existing suites. Detail-navigation fixture
exports retain the existing client models while adding the editor DTO imports.
Shared CSS, core rate/delegation code, schema and policy state are unchanged.

Formatter23849 passed with only module-order adjustments; whitespace, shell
and browser syntax checks passed. Commit/push60912 and draft creation5072
completed. Full native88044 runs on clean published d5dc1da. Runtime source
inspection is recorded, but complete adversarial fixture review and wider
composition are still pending; do not infer executable success from copying
the original regressions. The draft states these limitations.

Full native40587 also completed successfully on #2660eec1a4 with exit0 and
all compatible checks passed, including deployment/OIDC. Its PR body now
records that outcome after reconfirming its live head. Full51012 (#267),
25958 (b7836d7 composition),64831 (#268aef180f) and88044 (#269d5dc1da) remain
live; #268 production release and Clippy have passed. No duplicate check,
extraction merge or closure. Next review the retained editor fixtures, collect
these checks, then continue Project reads/delivery, tasks and hunk accounting.

### Project reader extraction and Reports composition verification

Full native51012 passed on #267e029a89; its PR body was updated after
reconfirming the live head. Full native25958 completed with exit0 and all
compatible checks passed on the wider Reports composition b7836d7. This
composition still uses the original #268 fixture types appropriate to its
identity foundation; reconcile the later standalone aef180f test-only
adaptation when composing again. Other architectures were not executed.
The #26864831 and #26988044 gates remain live; neither was restarted.

Completed scoped source review of #269's original canonical-field, manager,
catalog, concurrency, HTTP, component and browser fixture changes and runtime
boundary. No new critical or high finding was identified. Updated the draft
description, without claiming an independent review or full executable pass.
Its release and Clippy stages have passed; remaining checks are pending.

Created feat/scoped-project-reads in .worktrees/scoped-project-reads from the
shared reader/editor foundation7a2d61c. The original reader requires the
existing financial snapshot boundary #2320bb5721 for legacy fee reads.
Composed that prerequisite locally as c6e96f4 on
integration/project-read-prerequisites. The only manual merge resolution
united HTTP fixture registrations and calls, retaining both parents' suites.
No GitHub merge occurred; this new composition is not yet verified.

Extracted the2631186 ordinary reader, budget summaries, row projections,
requester-bound overview/detail UI and original database/component fixtures.
Preserved master's client-linked project route and passed initial_client into
the keyed overview child. Retained the shared is_admin helper because Clients
still consumes it. Adapted Client detail's existing spend caller and optional
amount display: withheld money remains unavailable, never fabricated zero.
Preserved all Client navigation fixtures and their deferred responses while
adapting the shared Project fixture; did not replace it with the older file.
The original new reader database fixtures and two component submodules were
read and retained without adding the later Task lifecycle fixture.

These reader changes are still local and uncommitted, not build-ready or
published. Whitespace checks pass, but HTTP/browser fixtures, SQLx inventory,
formatting, full native verification and remaining review are outstanding.
Project CSV/XLSX and Harvest-compatible list/count/direct-ID release boundaries
remain separate extractions. Next finish these reader regressions and caller
checks, publish the bounded draft, collect the two live gates, then continue
delivery, Tasks, unpublished Clients classification and full hunk accounting.

### Project reader draft published; editor fixture isolation corrected

Reconfirmed #216 merged at02f7b58. The preceding iteration was progress:
reader source, original fixtures and compatibility adaptations were preserved,
and the ledger was published as3d61f03. No unchanged check was restarted.

Full native64831 completed with exit0 and all compatible checks passed on
#268aef180f. Updated its PR body after confirming the live head. Other
architectures and retargeted GitHub acceptance remain separate.

Full native88044 on #269d5dc1da failed in the existing client-context browser
fixture after production builds, Clippy and earlier suites passed. Its global
draft oracle parsed multiple creators' rows as one JSON document. The retained
new-project-permissions fixture intentionally creates another user's draft.
Commit fdae71d scopes the oracle to the unique active DEV_LOGIN administrator
and organization. This matches the real login selection and the per-creator
draft constraint, retains every assertion and leaves other creators' drafts
untouched. No production code changed. Formatting traversal/whitespace passed;
the JavaScript file is not covered by treefmt, so traversal is not a JS syntax
proof. Published the fix and started full native87701 on the clean head;
updated #269 to record the failure and pending rerun, not a pass.

Published draft #270 at0723bb3, based on
integration/project-read-prerequisites c6e96f4. Its89 changed paths comprise
40 source/test/runner paths and49 SQLx descriptors. Retained the original
2631186 HTTP fixture and registered it alongside all existing checks.
Adapted the original browser fixtures to the bound overview and shared
task/team editor, retaining their failure/retry, rate, bulk-selection and
keyboard assertions. The new scoped Project-read fixture runs first; all23
previous suites remain. Modals and responsive fixtures preserve their actual
base's Timesheet and Client APIs rather than importing unrelated changes.

The scoped query inventory parsed259/259 macros after explicitly handling
Rust escaped line continuations. Restored49 missing descriptors only after
query/hash equality against2631186; no heuristic deletion. Core budget,
project DTO, backend reader and budget implementation match the original
source exactly. Formatting31397 passed with only component-fixture import
ordering; whitespace checks passed. Commit/push37975 and draft creation93585
completed; full native23221 runs on clean0723bb3. No executable acceptance
or final adversarial sign-off is claimed yet.

Project export links retain the original expected-requester parameters, but
CSV/XLSX server release guards and Harvest-compatible readers remain separate,
unextracted responsibilities. The draft explicitly records that limitation.
Do not activate policy or infer finished delivery enforcement from these links.

Next finish the scoped reader adversarial review, collect87701/23221, correct
any concrete failures, then extract project delivery/compatibility readers,
Task lifecycle/readers/UI, classify unpublished Clients and complete hunk-level
ownership and wider composition. No extraction PR was merged or closed.

### Project delivery extraction and concrete verification corrections

Full native23221 on #2700723bb3 failed Clippy after successful server/WASM
builds: the detail-navigation fixture re-exported an unused project_managers
module. Commit ed964fc removes only that unused module/import. Formatting88589
and whitespace passed; commit/push22660 completed. Full native56170 runs on the
published corrected head, and the draft now records both the failure and rerun.
Scoped reader review found no new critical/high issue in the extracted ordinary
read boundary: current organization/actor locks, strict stored policy/grants,
tenant joins, minimal labels, optional money, budget totals and original race
fixtures were inspected. This is not independent sign-off. Existing lifecycle
and fee operations still use their legacy role/snapshot gates; their canonical
authorization is explicitly outside this reader extraction and policy must
remain inactive. Wider UI/composition review and full acceptance remain pending.

Full native87701 on #269fdae71d passed the formerly failing client-context suite
and reached the history fixture. Its zero-receipt precondition failed because
the canonical editor fixture left four of its own receipts after deleting its
project. Commit8da2639 cleans only receipts joined by organization, actor and
request ID to that fixture project's edit requests, before cascade deletion.
It additionally checks that the original receipt snapshot is unchanged. The
history fixture and all prior assertions are retained. This cleanup runs only
inside the guarded disposable browser database, never a real database. Node
syntax93514 and whitespace passed; commit/push1458 completed. Full native89598
runs on the corrected head. The draft records this actual failure, not a pass.

Published draft #271b0cd080 on feat/scoped-project-exports, based on
integration/project-delivery-prerequisites abc7642. This base locally combines
#267e029a89 and #270ed964fc. Initial composition5f2eca0 resolved models, server
exports and two fixture registries by retaining both parents' registrations;
abc7642 then incorporates the reader test-import correction. No GitHub merge.
The base is an integration artifact, not an independent delivery PR.

The export extraction owns21 paths:16 Rust source/test paths and five original
SQLx descriptors. Source2631186 runtime and three new fixture modules are
preserved, with shared call sites receiving the monetary-context argument.
Requester binding rejects malformed/partial/mismatched identity without
selecting another actor. Canonical scope and money visibility are independent;
CSV source-state restoration applies to empty exports too. Workbook release
and CSV delivery after capacity waits recheck captured project/monetary access.
Retained nine canonical DB regressions, three parameter tests and legacy suites.
Scoped inventory parsed28/28 query macros and restored exact original cache
entries after query/hash equality; no heuristic cache deletion. Formatting26118
and whitespace passed; commit/push51395 and draft11523 completed. Full native82670
runs on the clean published head; live-schema acceptance remains pending.

Next collect56170/89598/82670 without duplicate runs, address concrete failures,
finish Project reader/delivery review, then extract the existing Harvest project
API list/count/direct-ID boundary. Created .worktrees/scoped-harvest-projects
on feat/scoped-harvest-projects from ed964fc for that bounded extraction; no
changes there yet. Tasks, unpublished Clients and complete hunk ownership remain
outstanding. No extraction PR was merged or closed, and no policy activated.

### CI repair priority and pending extraction verification

The user prioritized restoring Nixbot before further extraction work. Master
ed558f6 builds the application and passes its non-VM checks on both Linux
architectures, but Nixbot build138 fails both ARM VM checks before application
assertions: QEMU falls back to TCG and the test driver's fixed 300-second shell
connection timeout expires while the guest is still booting. The farm documents
TCG as intentional; no infrastructure changes or disabled checks are justified.

Draft #274, fix/nixbot-arm-vm-checks at cd26b56, adds a bounded serial-readiness
wait to the two existing VM tests. All application assertions remain unchanged.
Formatting and whitespace checks passed. Full native21618 and Nixbot build202
are pending; this is not yet a verified repair. No runner configuration, real
data or production application code was changed.

Before this priority change, draft #272 was published at bfa771d, based on
#270ed964fc. It extracts the original Harvest-compatible Project readers from
2631186: 16 paths, including ten original SQLx descriptors. The scoped query
inventory parsed21/21 macros and checked restored query/hash equality. Formatting
and whitespace passed. Full native46280 later failed the browser fixture
project-read-permissions.cjs at131: its errors array was nonempty. Diagnosis is
pending; compilation and Clippy success do not make this draft ready.

Draft #273 was published at571f3f5, based on #272bfa771d. It extracts original
task readers and their DB, HTTP and browser tests from f6e8bf1:18 paths, including
five original SQLx descriptors and the original three-line sidebar interaction
fix asserted by the browser fixture. Query inventory parsed58/58 macros, with
query/hash equality checked before restoring descriptors. Formatting, Node/shell
syntax and whitespace passed. Full native96635 remains pending. Canonical Time
commands are not in this base; activation still requires the wider composition.

Additional full native results are failures, not acceptance:

- #270ed964fc (56170) and #271b0cd080 (82670): new-project.cjs at1200 expected
  three options but received four. Fixture isolation needs investigation.
- #2698da2639 (89598): project-editor-permissions.cjs at145 expected two requests
  but observed one in the lost-acknowledgement retry scenario. The receipt cleanup
  has not received full-suite acceptance.

Do not rerun these unchanged heads or weaken their assertions. Next complete
#274's native and remote ARM validation, record its actual outcome, then return
to the concrete browser failures before further extraction. Existing original
branches/worktrees remain preserved; no extraction PR was merged or closed.

The initial #274 run exposed a second, test-driver-specific delay:
wait_for_console_text consumes one queued console line per one-second retry.
The guest had emitted readiness but the reader was still draining old boot
lines. Follow-up f869353 uses the existing bounded retry helper with
get_console_log instead; the readiness marker and900-second bound are unchanged.
Formatting and whitespace passed. Superseded native21618 was explicitly stopped,
not reported as success. Full native56305 and Nixbot build204 validate the new
head. The native OIDC VM completed its full assertions in21.47 seconds; the
remaining gates are pending. No application assertion or test was removed.

Full native56305 completed successfully on #274f869353: all native flake checks
passed, with the deployment/recovery VM completing in70.03 seconds. Nixbot204
is still pending and remains the required ARM evidence. Full native96635 on
#273571f3f5 failed in the existing new-project browser suite (expected three
project rows, received four); retain the draft and investigate isolation after
the priority repair. No unchanged extraction check is being restarted.

### CI repair follow-up and authorized branch refresh — 2026-10-07

Nixbot204 failed both ARM VM checks on f869353. The serial device dependency
timed out inside systemd after300 seconds, cancelling backdoor.service before
the driver's longer900-second readiness wait could help. Native checks passed;
this was not an accepted ARM fix.

A disposable local ARM VM using the exact cached guest kernel, initrd and
system closure reproduced the failure under TCG. /dev/hvc0 existed before
udev finished coldplug; systemd eventually marked it plugged after its job
had expired. Starting backdoor.service after coldplug immediately produced
the readiness marker. No real application database was accessed. The debug
VM was terminated after recording this evidence.

#274ccc2961 also sets DefaultDeviceTimeoutSec=900 inside the two test nodes.
Production service settings, assertions, supported systems and the overall
test deadline are unchanged. Full native97178 and Nixbot206 are running on
this correction; neither is yet reported as passed.

The user explicitly authorized merging #274 once the complete repair passes,
then rebasing the worked-on open PRs so their checks include the repaired CI.
No merge or rebase has occurred yet. Refresh stacks in dependency order,
preserve recovery refs and local edits, and use exact-head CI evidence after
each refresh. In particular #212 still has its18 original dirty paths; do
not overwrite them. This does not authorize merging extraction PRs or
continuing feature implementation ahead of the repair.

Full native97178 passed on ccc2961, including both VMs; deployment/recovery
finished in103.66 seconds. GitHub run37599025624 passed Flake Check and Format.
Nixbot206 failed before executing ARM checks: its scheduler reported no
connected, non-draining aarch64-linux worker after120 seconds. Both ARM VMs
and ARM formatting share this allocation failure; it is not evidence of a
test regression or a successful ARM correction. A single same-head Nixbot
rerun request through GitHub's documented check-run rerequest endpoint was
rejected with HTTP404 even though the same check is readable. It did not
schedule a new run.

One empty commit, #274a757709, triggers a fresh run without changing the
verified ccc2961 tree. Do not repeat empty commits if worker allocation fails
again; report the infrastructure blocker instead of bypassing ARM checks.

Nixbot reused failed build206 for the unchanged a757709 tree, so the empty
commit did not execute a new ARM run. The alternative check-suite rerequest
endpoint also returned HTTP404. GitHub Flake Check and Format passed on
a757709. A user-triggered Re-run on the
nixbot/nix-build check is now required to get fresh ARM evidence with the
available access. No CI result was overridden, no infrastructure configuration
was modified, and no PR was merged or rebased.

Read-only refresh audit: the52 extraction/ledger PRs #218–#273 all match their
published heads; only this ledger has current local edits. All other audited
extraction worktrees are clean. The read-only snapshot is saved locally in
`.scratch/pr-refresh-inventory-20261007.json`; re-fetch before any mutation.
Retain the57-open-PR inventory and dependency
base mapping when resuming; do not flatten shared integration bases into
unrelated feature diffs. Next action: validate the rerun of #274 on ARM,
merge only after complete acceptance, then perform the authorized refresh.

### PostgreSQL startup deadline in emulated VM checks — 2026-10-07

After the user requested another ARM diagnosis, direct Nixbot206 logs showed
new execution evidence despite GitHub still displaying the earlier failure.
ARM workers had become available:18 attributes succeeded and both VM checks
now reached the guest shell. The console device correction worked, but
PostgreSQL's120-second startup deadline expired during initdb. PostgreSQL
restarted successfully; Horae remained inactive because its dependency job
had already been cancelled. The worker-allocation blocker is superseded.

The user authorized fixing this. #2742d1486e sets only PostgreSQL
TimeoutStartSec=900 in both test nodes; TimeoutSec=120 still supplies the
unchanged shutdown deadline. Production modules, application assertions and
the overall VM-test deadline are unchanged. Source and whitespace review
passed. Full native43688 and current-head remote CI are running; no ARM
acceptance is claimed yet. PR metadata now describes the current failure,
not the obsolete allocation error. No merge or rebase has occurred.

Full native43688 passed on2d1486e, including both VM suites; the deployment
and import-recovery VM finished in129.29 seconds. The generated PostgreSQL
unit contains TimeoutSec=120 and TimeoutStartSec=900 as intended. Nixbot208
has completed18 of20 attributes and is executing the two ARM VM checks.
This is live validation, not the previous infrastructure blocker.

Nixbot208 succeeded on2d1486e for both Linux architectures, including both
ARM VM checks, after25m42s. GitHub Flake Check and Format also passed on the
same head. This supersedes the earlier blocked status: the test-only startup
correction is now fully verified. The user-authorized normal-queue merge of
#274 is being requested; confirm its actual merge before rebasing any stack.

#274 merged at11:43:02 UTC as8b3cc2577a3704cad28ee2e02a028fbf52668780.
Merge-group run37615829882 passed. The authorized branch refresh can now
proceed; this is not approval to merge any extraction PR.

### Post-repair stack refresh — 2026-10-07

Rebased all 52 extraction and ledger PRs in #218–#273 and their 18 shared
integration branches onto 8b3cc2577a3704cad28ee2e02a028fbf52668780. All 70
branches were published together with an atomic push and exact old-head
force-with-lease checks. No extraction PR was merged.

For every branch, the resulting tree exactly matches the original tree
combined with the repaired master. All 51 non-ledger PR diffs are byte-for-byte
unchanged at their respective merge bases, and every existing ancestor
relationship with a PR base is preserved. Historical integration conflicts
were resolved only after confirming equality with the original merge's
resolution. No application behavior was changed by this refresh.

Recovery refs remain under `backup/ci-refresh-20261007/`, with a verified
bundle in `.scratch/ci-refresh-before.bundle`. The local manifests
`.scratch/ci-refresh-plan.json`, `.scratch/ci-refresh-completed.log` and
`.scratch/ci-refresh-published.json` record original and published heads.
The original #208, #212 and #217 branches remain unchanged; #212 retains its 18
uncommitted paths. They are preservation sources, not refreshed extraction
branches. Root master and unrelated worktrees were not reset.

CI must validate the newly published heads. The successful #274 checks prove
the CI repair, not the correctness of every extraction. Existing browser
failures in #269–#273 remain unresolved until fresh evidence demonstrates
otherwise; those PRs remain drafts. Next action: inspect fresh-head results,
address extraction failures within their own scope, then continue the original
work-accounting and verification goal without adding features or merging
extractions.

### Project filter regression diagnosis after the refresh — 2026-10-07

Reconfirmed #216 merged at 02f7b58. The preceding iteration was progress:
#274 merged after passing both architectures and all extraction branches were
refreshed with verified tree and dependency preservation. No original branch
or unpublished Client work was modified.

On #270 at 18a2e4a, the existing native browser log identifies the fourth row
as the `Team recovery` project intentionally created by `action-errors.cjs`.
The `new-project.cjs` suite passes alone on the exact package
`8i4k07nmvmq6lx8xwxq2mp85lrl5zdkj-horae-0.1.0` (session 77388), establishing
that its fixed three-row assertion depends on suite ordering. This is not a
duplicate project produced by the lost-response creation retry.

Commit 2dbab02 in #270 changes only that test: capture the unfiltered project
identities and require both their exact count and identities after Reset
filters. Existing one-row tag filtering, empty search, disabled bulk actions,
creation replay, budget, report/export and invoice assertions remain intact.
The previously failing sequence `action-errors new-project` passes on a fresh
disposable database (81244). Node syntax and whitespace pass; treefmt does not
format this JavaScript file. Bounded adversarial review checked missing,
duplicated and substituted rows: the identity multiset comparison rejects
all three, unlike a count-only check. No production code, dependency or data
change was necessary. Full native gate 57812 is running on published 2dbab02;
neither complete acceptance nor downstream propagation is claimed yet.

The #269 canonical editor fixture passes in isolation on its exact package
`7r6v6zz9wq53cz7rmn8xmnycjx1fm6j2-horae-0.1.0` (1818), including the two
identical replay requests and unchanged revision. Its original failing log
still records one request instead of two. No speculative correction was made;
the intermittent failure remains unresolved pending stronger evidence.

Fresh remote evidence also prevents declaring ARM fully reliable across these
extractions. Nixbot build210 on #270's rebased 18a2e4a failed browser checks
and both ARM VMs. Its OIDC VM reached a listening Horae service around guest
second635 but never emitted the test-console connection marker within900
seconds. Its deployment VM did connect and reached the repeated-import
assertion, then exceeded wait_sql's90-second deadline waiting for `succeeded`.
These are distinct failures, not proof that PostgreSQL's startup correction
regressed. #272 build215 also reports failing ARM and browser checks; its
individual causes are not yet diagnosed. No limits or assertions were weakened.

Next inspect the live 57812 result, propagate the verified test correction
through its existing dependent stacks, and diagnose the two new ARM failures
before claiming reliable remote acceptance. Retain #269–#273 as drafts and
continue the original hunk-accounting goal; no extraction merge is authorized.

Full native 57812 failed in the project-only request audit, before the filter
suite: `directories` contained the legacy Timesheet's `list_clients` call from
the login landing page; `errors` was empty. The listener was installed before
login, and the old Timesheet explicitly mounts that directory resource. The
failure therefore depends on whether landing-page hydration starts before
the fixture navigates to Projects.

Commit 387f80f in #270 authenticates through the same dev-login endpoint using
the browser context's shared cookie store, with redirects disabled and the303
response and `/` destination asserted. No Timesheet is mounted; the directory
listener remains active before the first Projects navigation and none of its
assertions are removed or filtered. All project browser interactions, grant
revocations, inactive-account checks and requester-binding checks remain real.
Focused Chromium 8195 passes all three scenario groups; Node syntax and
whitespace pass. Full native 36963 is running on published 387f80f. Do not
restart the completed failing 57812 or count the focused pass as full acceptance.
Downstream propagation waits for this corrected head's full result.

### Detail browser contract and remaining blob inventory — 2026-10-07

The intervening estimate-only reply was no progress. Reconfirmed #216 merged
at 02f7b58 before resuming changes. Native gate36963 is terminal failed, not
still running: both earlier browser fixes passed, then
`new-project-permissions.cjs` attempted to replay `get_project_details`, which
the current detail UI no longer calls. The same fixture also assumed separate
assignment/task requests and an error inside a retained detail region.

Commit 4a5c7d9 in #270 adapts that browser matrix to the real
`get_project_detail_view` request and its nested project/label contract. It
asserts the exact requester, legacy-policy binding, edit affordance, project
identity, complete team/task labels and absence of financial fields even for
administrators. All six role/visibility cases, private-note redaction, foreign
organization denial, creation/draft ownership, exports, reports, revocation,
inactive users and anonymous creation requests remain exercised. Denied detail
must remove the details/team/tasks regions, and private notes must be absent
from the entire rendered body, not only one region. No production code changed.

The underlying legacy readers remain covered outside the browser through
`members_can_list_identities_but_not_assignment_rates`,
`managers_and_admins_can_read_assignment_rates`,
`revoked_membership_preserves_only_own_historical_tracking_identity`,
`project_reads_use_current_authority_and_reject_foreign_scope` and the registered
HTTP tests in `authorization_tests/project_reads.rs`; adapting the UI fixture
does not remove those tests or endpoints. Focused Chromium50335 passes every
matrix group on the exact unchanged production package from387f80f. Node syntax
and whitespace pass. Full browser sequence10797 is running on the same package
with the published fixture; complete fresh-head acceptance remains pending.

A read-only blob comparison is saved in the repository scratch artifact
`permission-blob-accounting.json` (generated by its sibling `.mjs` script).
Against original #212 db3935d and its9301112 base, 1,026 of1,214 changed paths
have an identical blob in at least one extraction:752 SQLx descriptors and274
other files. Another168 paths require adaptation/retained-work review
(69 SQLx,99 other files);20 original SQLx deletions need explicit query ownership
review. These numbers are not feature completion or final ownership: inherited
blobs may occur in multiple dependent PRs, and differing shared files can be
valid adaptations. #217 and the18 preserved dirty Client paths are separate
inventory obligations, not included in these counts. Nothing was deleted.

Next finish the browser sequence and fresh-head gate, propagate verified test
adaptations through #272/#273 and #271's shared base, continue diagnosing the
distinct ARM failures, and resolve the remaining hunk ownership. Task lifecycle
and catalogue UI work remains preserved in #212; it is not silently discarded
or counted as an accepted extraction. No merge or policy activation performed.

The scratch comparison now records the original deleting commit and every
extraction where each deleted descriptor is absent. Seven of the20 deletions
are reproduced by an owning extraction (and sometimes inherited downstream):
`074eb86a0a23` and `4fa0a9d60b98` belong to #237's requester retention;
`9ff1b7d9e9c8` and `ed1c72f60026` to #228's lock ordering;
`548235be5387` to #249's download release checks;
`717614d03bf8` to #247's project export sizing;
`bcde45c163fc` to #263's spreadsheet sizing. Prefixes identify the original
`.sqlx/query-<hash>.json` paths in the full scratch manifest.

The remaining13 descriptors still exist in the extraction trees. Their deletion
responsibilities are now explicit; this is not approval to remove them without
checking each owner's current SQLx inventory and any composed consumers:

| Owner | Original descriptor hash prefixes | Responsibility |
| --- | --- | --- |
| #270 | `22bd5f392be7`, `6bd34bcfb204`, `baf75ef8ba55`, `bc3f9ea5c9aa`, `c1f2843a6aaf`, `e25b27423bd4` | Project details, budget, spend, list, assignments and tags replaced by scoped queries |
| #272 | `1f0c682793eb`, `40899460e93f`, `4c92eb5086da` | Harvest project count, direct lookup and list |
| #273 | `3036fe659e73` | Harvest task count |
| #269 | `a1ea432d7440` | Project editor settings update |
| #276, original0591407 | `2a22de0eb8e0`, `8aae7affad63` | Superseded by activity-aware checkpoint restore/import inserts in #276; original descriptors remain recoverable in the parent and preserved source |

Fresh-head native gate51216 is running on #2704a5c7d9. GitHub initially
rejected the ledger push and PR-body update with internal server errors; the
remote branch and body were checked before retrying. This transient publication
failure did not stop local verification or modify the original branches.

The second ledger push and GraphQL update also failed; the subsequent REST
body update returned an empty/invalid response and must be read back before any
retry. Ledger commits e510957 and866ce28 remain local until publication is
confirmed; #270's code commit4a5c7d9 was published successfully beforehand.

Verification boundary: #269 and #270 are sibling extractions on the common
reader/editor foundation, not parent/child. The #270 default browser runner
does not include #269's `project-editor-permissions` fixture, which is absent
from its source tree. Its passing legacy `project-edit` checks cannot close
#269's canonical editor failure. Their later integration must combine both
fixture registrations and verify both contracts; no suite was removed in this
iteration.

Browser sequence10797 completed successfully with all default suites, including
the corrected creation-permission matrix, editor/navigation checks, Clients and
own-permission/history suites. It used the production binary from387f80f with
4a5c7d9's test-only changes; native51216 verifies the actual new source head and
remains live. REST readback confirms the PR body was not updated, and the ledger
remote is still f6e29c8. Do not claim these local ledger commits are published.
Next collect51216 without restarting it, then propagate the verified fixture
commits to dependent stacks and publish the retained ledger updates once GitHub
accepts writes. The unresolved ARM and sibling-editor gates remain separate.

### Verified project parent, propagated stacks and task commands — 2026-10-07

The preceding goal turn made progress: published browser contract correction,
passing focused/full browser checks and explicit original SQLx deletion owners.
Reconfirmed #216 merged and original #212 still at db3935d with the same 18 dirty
paths before continuing. No original or #208 files were changed.

Full native gate 51216 completed with exit 0 on published #270 4a5c7d9: all
x86_64 checks passed, including the full browser runner, SQLx, server/WASM,
lint, tests and deployment/OIDC checks. Its application unit suite reports
1,066 passed, zero failed and 11 inherited ignored; component/integration suites
also passed. The command explicitly omits incompatible ARM/Darwin systems, so
this is not ARM acceptance. Current Nixbot 266 is still running on that head.

Propagated only the three verified browser-file changes through the existing
dependent stacks, with expected trees calculated by `git merge-tree` before
updating any branch and compared exactly afterward:

| Branch / delivery | Before | After |
| --- | --- | --- |
| #272 `feat/scoped-harvest-projects` | abe5aa0 | fd91c3d |
| #273 `feat/scoped-task-reads` | 5a04f89 | 4b2c87d |
| `integration/project-delivery-prerequisites` | 8cd7a32 | 375e9bd |
| #271 `feat/scoped-project-exports` | 255aa94 | d61bfd3 |

The prerequisite branch uses a local composition commit; no GitHub PR was
merged. All four refs were published atomically with exact old-head leases.
The manifest is `.scratch/project-browser-propagation.json`; original heads
are preserved under `backup/project-browser-20261007/`. The global Git
`rebase.updateRefs` option initially moved three newly created backup branches;
they were restored using compare-and-swap to the manifest's original hashes
and reverified before publication. The scratch propagation script now disables
that option. No original work was lost or rewritten. Fresh full native checks
76300 (#273, including #272) and 81807 (#271) are running; old passes are not
counted for these new heads.

Draft #275 at 9548cdd owns the existing creation and direct-edit commands from
8dd61d4/facfb49 on #273 4b2c87d. Its 33 paths preserve 11 creation tests, eight
edit tests, strict transport tests, legacy mutation regressions and both real
session matrices. Six complete source files, the task command/mutation-test
sections and 22 query descriptors match facfb49 exactly; descriptor SHA-256
hashes were verified. The shared project mutation fixture deliberately retains
this base's editor request shape instead of importing #269's separate fields.
HTTP registrations were unioned and formatted in the existing harness.

The original task contract is retained with its historical source-snapshot
acceptance records, not claimed as a new test result. This extraction preserves
policy-zero behavior and adds no activation, migration, UI, dependency or real
data change. It replaces one obsolete task-link cache descriptor only in this
new worktree; the descriptor remains recoverable in the parent/original history.
Formatting and whitespace pass. Fresh tests and live-schema SQLx validation
are running in Nix session 53475. Full gates and final adversarial review remain
pending, and the PR is explicitly a draft.

The later original task activity/import work, existing-task links, catalog UI,
atomic creation with a rate/requester and project-editor archive/restore remain
separate pending extractions; #275 does not claim to deliver them. Next collect
76300, 81807 and 53475 without restarting live checks, complete #275's gates and
review, reconcile the remote descriptions/ledger after the publication errors,
then continue those remaining task boundaries and full hunk accounting. No
extraction merge, #212/#217 closure or policy activation is authorized.

### Task cache correction and verified project dependants — 2026-10-07

The preceding estimate-only response was no progress. Reconfirmed #216 merged,
checked the original worktree still has its 18 preserved dirty paths, and polled
the existing process handles before taking action. No original files changed.

Gate53475 terminated with three missing SQLx descriptors in task creation tests;
the live-schema prepare check independently reported the incomplete cache.
Published unsigned commit f8c1477 in #275 restores descriptors6b3f433,81aefb2
andbe0c828 verbatim from originalfacfb49. No Rust or assertion changed. Fresh
gate96425 has passed live-schema SQLx (with the inherited unused-query warning)
and is executing tests; full native and final cross-PR gates remain pending.

Source review of #275 traced session identity through organization/actor locks,
current strict grants, project designation, atomic linked creation and task-row
serialization. Inspected all 11 creation and eight edit regressions plus both
registered-session matrices. Explicit rate intent authorizes even financial
no-ops, validates current currency under the organization lock, preserves
overrides/history and separates committed event payloads from redacted replies.
No new critical/high issue identified in this bounded source review. This is
not independent signoff or acceptance of later lifecycle/link/catalog commands:
those retain legacy behavior here and policy activation remains forbidden.
Creation requester/rate transport from laterdcadcee is still separate work.

Full native gates76300 (#2734b2c87d) and81807 (#271d61bfd3) terminated with
exit0 and all checks passed, including browser and deployment/OIDC. Both
explicitly omitted ARM/Darwin. #272fd91c3d now has its own exact-head full
native gate50478 running; the passing child composition is not substituted
for it. Successfully updated PR descriptions270–273 and275 with current
evidence; the previous GitHub publication failures no longer block these edits.
Ledger28f61fe was also successfully published before this iteration.

Nixbot266 on #2704a5c7d9 has now terminated with ARM deployment failure:
the restart-import scenario waited93.77seconds for an advisory-lock waiter at
scriptline154 (90-second bound). The existing log does not expose the job's
status or database wait reason at failure; it cannot distinguish slow progress
from a failed/stuck job. This differs from the earlier repeated-import wait.
No timeout or assertion was relaxed, and ARM acceptance is still unproven.

Regenerated the private original-blob inventory including #275: 1,049 of1,214
paths match an extraction exactly,145 need adaptation/retained-work review,
and20 original deletions still require ownership reconciliation. These are
conservation clues, not feature completion percentages or a final hunk audit.
Next collect96425/50478 without restarting live work, finish #275's full gates,
then continue the retained task boundaries and shared-file ownership audit.

The corrected #275 cache provenance check now covers all25 added descriptors:
every file matches originalfacfb49 and its query SHA-256. The remaining lifecycle
boundary was traced before choosing another extraction: ac4c90c changes global
activity, while0591407 adds migration0048, independent project-link activity,
editor intent/receipts, tracking admission and import/checkpoint preservation.
Taking only the first commit would omit the original restore guarantees.
Its composition must include #275, the #269 editor and the #258 time-command
boundary; import integration must also preserve #223/#224/#231. None of these
five sibling branches is an ancestor of #275, confirmed by Git ancestry checks.

A non-mutating worktree preflight of #275f8c1477 plus #269a9ba27b produced two
conflicts: the browser-suite registration list and detail-navigation tests.
HTTP registrations, task mutation tests and the new-project permission fixture
merged textually, but still require semantic review; an automatic merge is not
evidence of compatible contracts. No branch/worktree was merged or changed by
this preflight. Resolve those test unions explicitly when composing the next
boundary, retain all suites, and keep #269's unresolved lost-ack fixture visible.

Gate96425 subsequently completed with exit0 on publishedf8c1477: 1,117
application unit tests passed, zero failed and11 inherited ignored, plus all
component/integration suites. All19 creation/edit regressions and both
registered-session matrices pass. Full native gate71588 is now running on the
same unchanged head; gate50478 for #272 also remains live. PR275 now records
the completed narrow gates and pending full acceptance. The latest ledger
publication before this update wasb4e7346. Next collect71588/50478 and retain
the separate ARM and editor-fixture blockers; do not restart live checks.

Gate50478 then completed with exit0 and all x86_64 checks passed on #272fd91c3d.
ARM/Darwin were explicitly omitted. Updated PR272 accordingly. Ledger04048fb
was published successfully; only full #275 gate71588 remains live from this
iteration, currently running Chromium. Next collect that handle, then continue
the lifecycle composition and unresolved cross-PR/ARM verification.

### Lifecycle prerequisite integration — 2026-10-07

The estimate-only response was no progress. Revalidated #216 as merged at
02f7b58 and the original worktree's 18 preserved dirty paths. Full native gate
71588 passed on #275 f8c1477; its PR description now records the result.
ARM acceptance and the wider separation review remain pending.

Published integration/task-lifecycle-prerequisites at 1955c38. It combines
#275 f8c1477, #269 a9ba27b, #258 e1525af, #223 1dc4b6b, #224 8ffac9d and
#231 2e4bbf9 without merging any GitHub PR. Resolved test-registration and
model-export conflicts as unions; retained all 26 browser suites and 18 HTTP
matrices. The private composition audit verifies all six ancestors and 705
unchanged single-owner blobs, and inventories 25 shared paths. Descriptor
c5179dc remains absent because #275 replaced its query; the composed projects
module matches #275 exactly. These checks are conservation evidence, not final
semantic acceptance.

Full native gate65768 is confirmed live on unchanged1955c38, currently executing
browser tests. Do not modify or restart that worktree while it runs. The clean
child feat/scoped-task-lifecycle is based on1955c38. Next extract original
ac4c90c and0591407 together (global/project activity, migration0048, editor
intent, tracking admission and import/checkpoint preservation), retaining
later catalog/link/editor-control work separately. Collect65768 and verify the
new extraction on its own head before claiming acceptance.

### Task lifecycle extracted and prerequisite gates passed — 2026-10-07

Published draft #276 on feat/scoped-task-lifecycle. Original ac4c90c+0591407
were extracted together in924dcbe. All67 changed-path added/deleted line sets
match facfb49..0591407 except the intentionally excluded historical progress
log; the HTTP registration contexts were adapted to preserve all18 existing
matrices and add task_activity as the19th. All39 added files are byte-identical
to the source, including34 query descriptors with matching SHA-256 hashes.
Removed five superseded descriptors only in the extraction; each remains
recoverable from its parent and the preserved original. The two previously
unowned deletions2a22de0 and8aae7af now belong to #276's activity-aware import
and checkpoint inserts.

Gate72726 failed on one missing descriptor for the existing organization-lock
NOWAIT regression. Corrective unsigned commit7e883eb restores2889c080
verbatim from original0591407; no Rust, query or assertion changed. Both commits
are published. Fresh tests/live-schema SQLx60667 and full native95578 are live
on unchanged7e883eb. Do not edit this worktree while either check runs.

Full native gate65768 finished with exit0 on prerequisite1955c38. All x86_64
checks passed, including browser, PostgreSQL suites and deployment/OIDC;
ARM/Darwin were explicitly omitted. Its main app suite passed1,210 tests with
zero failures and11 inherited ignored. The successful combined browser run is
new evidence, not an explanation or permanent fix for #269's earlier lost-ack
fixture failure. Keep that investigation and ARM deployment acceptance open.

Bounded lifecycle source review traced current actor/grants under organization
and actor locks, stable project-before-task locking, running-timer exclusion,
rate redaction after constructing committed event payloads, project authority,
protected-field preservation and replay identity, tracking admission, and old
checkpoint/default handling. Reviewed seven global activity tests, three editor
activity tests, the real session matrix, migration, importer/lock-order and
tracking regressions. No new critical/high source issue identified within this
boundary; fresh full and cross-PR gates still determine acceptance.

The next original link increment979a594 depends directly on this lifecycle and
editor base; its canonical rate-currency rule must be kept alongside authority.
Catalog5561f14, atomic creationdcadcee and editor controls8c1bf9b remain separate
retained work. Next collect60667/95578 without restarting live processes,
continue those extractions, and finish shared-hunk/deletion and unpublished
Clients accounting. No GitHub merge, original closure or activation performed.

The refreshed private blob inventory includes #276 and identifies1,103 of1,214
original paths with an exact extraction blob;91 require adaptation/retained-work
review and20 are original deletions requiring separate ownership review. The
two #276 deletion owners are now documented above. These counts are not a
completion percentage and do not replace the shared-hunk or dirty-work audit.

### Existing-task link extraction — 2026-10-07

The previous goal iteration made progress: published #276 and its original
cache correction, verified the six-parent native composition, and published
ledger282813e. Reconfirmed #216 merged and the original18 dirty paths before
continuing. Original backup refs remain db3935d and d364270.

Created feat/scoped-task-links in .worktrees/scoped-task-links directly on
#2767e883eb. Unsigned commitd1ab522 extracts original979a594 without new
functionality. All14 changed-path added/deleted line sets match the original;
all nine added files are byte-identical, including seven query descriptors with
verified hashes. The HTTP registry preserves19 parent matrices and adds
task_links as the20th. Descriptorfa78a5ad is superseded only in this extraction
by the currency-aware query and remains recoverable in its parent/source.
Historical progress/quickstart/task-status changes stay with #248/original refs.

Bounded source review followed both enable_project_task callers, requester
binding, current all/managed project authority, independent explicit-rate
authorization before existing-link no-ops, organization/actor/project/task
ordering, tenant joins, integer parsing, canonical default currency even without
project_settings, and archived/history preservation. Retained all five original
database regressions and the nine-case HTTP matrix, including actual lock-wait
revocation and stable-denial/no-op storage checks. No new critical/high finding
identified in this boundary; full and cross-PR acceptance remain pending.

Formatting and whitespace pass. Fresh tests/live-schema SQLx gate23598 is live
on unchangedd1ab522. Publication attempt74103 has reported SSH-agent signing
refusal and has not completed; do not claim a published branch/PR yet. The
prepared draft body is retained in the repository scratch directory. #276's
body-update handle62777 is also still live; readback confirms its head7e883eb
but not the revised body. Do not repeat a pending write solely for lack of output.

#276 gate60667 has completed live-schema SQLx validation and is running tests,
including successful import activity/checkpoint lock-order regressions. Its
full native95578 remains live. Next collect these exact handles, resolve any
terminal publication failure using existing authorized credentials, and publish
the link PR with accurate evidence. Keep original catalog5561f14, atomic
creationdcadcee, editor controls8c1bf9b and unpublished Clients as remaining
work. ARM and the earlier intermittent editor fixture remain unclosed.

Publication74103 subsequently completed successfully: branchd1ab522 and draft
#277 are published after SSH tried another configured identity. No credential
change or duplicate push was needed. Gate23598 passed live-schema SQLx (with
the inherited unused-query warning) and has started the test derivation. #276's
tests60667 and full native95578 remain live. Ledger82d512f was committed locally
before this publication result; publish it together with this update. The
separate #276 body update62777 is still unconfirmed and must not be repeated
while its handle remains live.

Gate60667 subsequently completed with exit0 on #2767e883eb: live-schema SQLx
and every database/component/integration suite passed. The app suite reports
1,224 passed, zero failed and11 inherited ignored. Full native95578 remains live;
this narrow pass does not close ARM or browser/deployment acceptance. #277's
gate23598 has passed live-schema SQLx and is compiling its application tests.
The ledger through0ec9a24 is published successfully.

Refreshed private conservation inventory including #277:1,113 of1,214 original
paths have an exact blob in an extraction;81 require adaptation/retained-work
review and20 are original deletions with separately tracked owners. Do not
interpret this as a completion percentage. Next collect95578/23598 and the
still-live #276 body update62777, then start #277's full native gate when the
current builders release resources. Continue with the original catalog plus
atomic creation and separate editor controls; never alter a checked worktree
while its gate is running.

### Atomic task creation and catalog prerequisites — 2026-10-07

The preceding estimate turn yielded new evidence: #277 tests/live-schema SQLx
23598 finished with exit0, and publication85197 completed with draft #278.
The current iteration reconfirmed #216 merged at02f7b58 and the original dirty
worktree unchanged. No GitHub merge, closure, activation or real-data change.

#278 at3d76f9968496ffe38ade1405265f737de15cc39e contains dcadcee's backend,
legacy caller, DB/HTTP tests and cache. All12 changed-path line sets match the
selected original delta; all five new descriptors are byte-identical with
valid query hashes. The replaced insertion descriptor remains in source/parent
history. Current identity, independent explicit-rate authority, workspace
currency and optional project authority are checked before an atomic insert;
events retain committed values while response projection respects rate access.
No new critical/high finding in the bounded source review. Tests/live-schema
SQLx63668 is running; full native and ARM acceptance are still pending.

The legacy caller adaptation adds requester loading inside the existing
AdminUsers form, whereas the original calls that form LegacyAdminUsers after
#260. Catalog work therefore needs the actual #260 shell/People dependency,
not a duplicated partial shell. Created integration/task-catalog-prerequisites
from #278 plus #260f00d8f0. Explicit merge resolutions retain the requester in
LegacyAdminUsers and the union of both browser registries. This is a local
verification composition, not a delivery PR or a GitHub merge. The catalog
worktree will extract5561f14 plus dcadcee's remaining UI and original contract
appendices on that composition; project activity controls8c1bf9b remain separate.

#276 body update62777 finished successfully. Its tests60667 passed; full95578
is running. #277 full62455 is running after tests23598 passed. PR descriptions
were updated successfully in36369 with those exact-head results. Native
browser progress does not close the ARM import-checkpoint failure or diagnose
#269's intermittent fixture. Next finish the composition audit, extract the
catalog, collect existing live gates and continue Clients/shared-hunk accounting.

### Catalog extraction and completed native gates — 2026-10-07

Published #279 at46d3b363e8e46a5619e9746a719ab03886b08d0f on the published
0b781f94238f6228e24fd57eac252219279972e6 composition. Source audit retained
780 single-owner blobs, inventoried26 shared paths and found no missing blobs;
the previously superseded c5179dc descriptor remains accounted for. The two
merge resolutions preserve #278's requester inside #260's LegacyAdminUsers and
the union of29 browser suites. All20 parent HTTP matrices remain registered.

#279 contains21 changed paths:5561f14 plus dcadcee's UI and original contract
appendices. All20 non-registry added/deleted line sets match their selected
source increments, all nine added files are byte-identical, and all three SQLx
query hashes match. The adapted registry preserves all29 parent suites and adds
task-catalog; the HTTP catalog matrix is the21st. Formatting/whitespace and
JavaScript/shell syntax pass. Tests/live-schema SQLx56599 is running; no own
full/browser or ARM acceptance is claimed. The contract's old verification
references remain historical, explicitly distinguished in the draft PR body.

Bounded review covered the current requester and grants under ReadAccess,
tenant filtering before pagination, separate rate projection and affordances,
non-authoritative cursors, stale UI suppression, modal/busy/validation handling,
and exact zero/blank/clear intents. Retained two DB regressions, six HTTP grant
cases, eight actual-component tests, independent shell authorization and the
original real-session desktop/mobile Chromium fixture. No new critical/high
source finding within this boundary. No CSS framework or product redesign.

Full native95578 on #2767e883eb and62455 on #277d1ab522 both finished exit0,
including browser and deployment/OIDC. ARM and Darwin were explicitly omitted.
Descriptions updated successfully in49930. Dependency full4427 remains live;
it reported an unfulfilled TaskRateEdit dead-code expectation because #278 now
has a browser caller. #279 already includes the original annotation removal;
the parent needs a small earlier cleanup after its live test63668 finishes.
Do not modify checked worktrees or treat native success as ARM acceptance.

The refreshed conservation inventory has1,134 of1,214 exact extraction blobs,
60 adaptation/retained-work paths and20 separately tracked deletions. These are
not completion percentages. Original dirty Clients work remains preserved.
Next collect63668/4427/56599, resolve the parent lint annotation, then extract
8c1bf9b's project task-activity controls and finish Clients/shared-hunk accounting.
ARM and the #269 intermittent fixture still need their own diagnosis.

Gate63668 subsequently finished exit0: tests and live-schema SQLx passed on
#2783d76f99. With no check left running in that worktree, unsigned812a870
removes only the seven-line TaskRateEdit lint expectation, moving the original
5561f14 cleanup to its first browser consumer. Fresh full native63014 is live
on that new head. Do not transplant it into the running composition4427 or
catalog56599 yet; propagate after those handles finish and verify the resulting
heads. No runtime/query/test assertion changed, and old results are not claimed
for the new commit.

### Project task controls and original dirty client reader — 2026-10-07

The previous iteration made progress: published #279, verified #276/#277 native
gates and preserved the scoped #278 lint correction. Reconfirmed #216 merged
at `02f7b58` before this iteration; originals and their 18 dirty paths remain
unchanged. No GitHub merges, closures, policy activation or real-data mutations.

Published draft #280 at `24c5d0e19b076aac28a92bd8d245159637309db4`, directly on
#276. All four changed-path line sets match original `8c1bf9b`; only context
was adapted around the parent's receipt-isolation fixture cleanup, which stays
intact. No dependency on the separate catalog screen or atomic creator is
needed. Review traced retained task identity, staged/undo/dirty state, frozen
pending receipts, definite rejection vs uncertain retry, disabled archived
fields/bulk billing, current project authority and server lifecycle checks.
Original running-timer, hidden-setting, revocation, keyboard and narrow-layout
regressions remain. No new critical/high source finding within this boundary.
Formatting/whitespace/JavaScript syntax passed; full native `14092` is live.

Verified all six original untracked client files byte-for-byte against their
tar backup; tracked original work still matches snapshot `d364270`. A probe
confirmed that stash-shaped snapshot has no third parent: untracked recovery
is the tar, not an invented Git ref. Created `feat/scoped-harvest-clients` on
#272 and committed `1a1de4afeb72fb1c1c8bc8b64dfa653fe5aebcac`. All five tracked
delta paths and six new files match the preserved source, including the three
query hashes. Three replaced descriptors are removed only in the extraction
and remain recoverable in source/parent history. The original worktree is intact.

Client review followed both list/detail callers, page validation, authorization
and actor fences, same-statement tenant-filtered totals, exhausted pages,
projection and release. All six original registered-session DB regressions and
legacy pagination checks remain. No new critical/high finding within this
reader boundary. Formatting passed; tests/live-schema SQLx `28333` is running.
Publication `2339` failed with GitHub internal server errors on both push and
PR creation. Read-only remote reconciliation `17844` is pending; do not invent
a PR number or repeat a write until its result is known. The provisional local
inventory was regenerated without an unconfirmed PR label.

#279 tests/live-schema SQLx `56599` finished exit0, including its eight actual
component scenarios and two shared model tests. Body update `63600` is pending.
#278 full `63014` and dependency full `4427` remain live. Propagate `812a870`
only after the dependency gate finishes, then verify the resulting catalog head.
Conservation inventory: 1,135 exact extraction blobs out of 1,214, 59 paths
requiring adaptation/retained-work review, and 20 separately tracked deletions.
This is not a completion percentage. Next collect these handles, reconcile
client publication, finish shared-hunk/specification ownership and diagnose ARM
and #269's intermittent fixture. T238 remains explicitly unfinished.

### Final task heads and client test-fixture dependency — 2026-10-07

The preceding estimate-only response made no implementation progress. This
iteration revalidated the state and resumed safe work. #216 remains merged at
02f7b58; the original18 dirty paths still match their preserved snapshot.

Full native63014 (#278812a870),4427 (composition0b781f9) and14092 (#28024c5d0e)
all finished exit0, including browser and deployment/OIDC. ARM/Darwin were
omitted. Only after these handles became terminal, the composition incorporated
#278812a870 as e9f793d. Its audit again retains780 single-owner paths,29 browser
suites and20 HTTP matrices, with no missing paths. The26 shared paths still
require final cross-PR accounting. The catalog incorporates that composition
as9fcc0a0, with tree fa15e37c3562215faf2e0373a2c85edfc0bb3ae2 identical to its
previous46d3b36. Fresh full native66969 runs on the new head; no historical
pass is substituted for final-head verification.

Client gate28333 failed compilation: sibling tests could not access the
canonical/selection fixtures. Unsigned1c254582 reuses exactly the original
f6e8bf1 pub(super) increment, already present in #273, without pulling unrelated
task runtime into the client reader. Production code and assertions are
unchanged. Formatting passed; fresh tests/live-schema SQLx65280 is running.
This shared test-only ownership must be retained when composing #273 and the
client extraction. Seven incomplete dirty client specification files remain
preserved and T238 is not claimed complete.

Prior publication retries2021/45046 failed with GitHub internal server errors.
#279 body writes63600 and92419 also failed; a fresh remote read85051 confirms
the old body remains and there is still no client PR. Publication91690 now
attempts the corrected client head and catalog/base updates; reconcile its
terminal result before repeating writes. Next collect65280/66969, publish
accurate descriptions and this ledger, verify the updated composition, and
finish shared-hunk/specification ownership plus ARM/#269 diagnosis. No GitHub
merges, closures, policy activation or real-data changes occurred.

Publication91690 finished successfully: catalog9fcc0a0 and compositione9f793d
are remote, and Clients is confirmed draft #281 at1c254582. PR descriptions for
#278/#279/#280 updated successfully in42193. Fresh composition full31756 is
running on e9f793d. Refreshed inventory includes #281 and still reports1135
exact blobs,59 adaptation/retained paths and20 original cache deletions.

Client65280 then failed live-schema SQLx validation for a missing descriptor,
after compiling successfully with the fixture correction. Offline tests11982
are diagnosing the exact consuming query; no cache is removed or assertion
weakened. #281 remains draft until both SQLx and the full gates pass.

Full native66969 subsequently finished exit0 on #2799fcc0a0, using cached Nix
outputs for identical derivations and checking the final source. This is native
acceptance only; ARM/Darwin were omitted. #279 body update67803 records it.
Client diagnostic11982 finished exit101 with exactly the missing metadata for
`UPDATE clients SET active=false WHERE id=$1` in the original client test.
Unsignedcd8d1db7 restores descriptor81aefb2e byte-identically from original
02c4245/db3935d; no new query or assertion. This is shared test-cache ownership,
not a runtime Timesheet dependency. Fresh full native15935 runs on cd8d1db7.
Next collect15935/31756 and complete the remaining source/shared-hunk and
specification reconciliation, ARM checkpoint and #269 fixture diagnosis.

### Shared-source conservation findings and ARM diagnostics — 2026-10-07

The previous iteration made progress: #279 final-head native acceptance and
#281 publication/corrected test dependencies. Reconfirmed #216 merged before
editing; original #212 tracked work still exactly matches its saved snapshot.
No original, #208, production data or policy state was changed.

Fetched the authoritative Nixbot266 ARM VM log to
`.scratch/nixbot-266-arm-e2e.log`. It proves the SIGTERM checkpoint, recovery,
totals/attempts/report and repeated import completed (the latter in84.72s).
The later SIGKILL setup timed out before observing its second-batch lock waiter
(93.77s for a90s bound). It does not expose that job's progress/error/lease or
blocking state. This does not prove an application deadlock or a sufficient
deadline. No timeout was relaxed.

#270737aa13a adds only bounded, best-effort failure diagnostics to wait_sql:
synthetic job status/progress/lease/errors, PostgreSQL waits/blockers/advisory
locks and an80-line service journal excerpt. It rethrows the original failure
even when diagnostics fail. A focused check of the evaluated Python helper
passes success, failure and unavailable-diagnostics cases, retaining90s wait
and15s diagnostic command bounds. Formatting passed; full native30162 then
passed exit0. Nixbot295 is confirmed running on that exact head. This is not an
ARM repair or acceptance claim, and the diagnostic has not been propagated to
dependent extractions. PR body76913 updated successfully.

Added a local triage script and artifact, `permission-shared-line-audit.mjs/json`,
which compare original additions with PR review deltas and current master.
It explicitly is not semantic, ordering, deletion or unique-ownership proof.
The first run exposed a missing-master-file handling bug in the audit script;
the corrected run handles original added files and uses origin/master for
independent PR bases. No extraction was changed by the inventory itself.

Manual review of its results found two real omissions, now corrected:

- #2715beac2e1 restores original2631186's complete project_requester HTTP matrix
  byte-for-byte and registers it in exports::check. Both CSV/XLSX requesters,
  anonymous/changed/foreign sessions, partial identities, headers and payloads
  are tested. No production query or assertion changed. Formatting and byte
  comparison pass; fresh full native37940 is running.
- #2681be11e74 restores ecac66b's exact six-line chevron-right/down match arms,
  used by grouped report expansion. Unknown icons previously rendered nothing.
  Two existing Chromium scenario assertions now require the actual collapsed
  and expanded SVG paths; accessible expanded state and all prior checks remain.
  Formatting/JavaScript syntax pass; fresh full native31708 is running.

Both corrections and their draft descriptions are published (29478/33527).
Their prior-head green checks are not final-head evidence. The refreshed blob
inventory contains1136 identical extraction blobs,58 adaptation/retained paths
and20 original cache deletions; these are not completion percentages.

Some apparent missing lines are already explained by checked extraction
boundaries: the original scoped_time legacy invoice-identity matrix is separately
registered as time_entry_payload::check in #242; the original user_directory
canonical matrix is registered as scoped_directory::check in #253. The original
configure_administration call in legacy time writers is inlined, with identical
SQL, in #241 to avoid an unrelated canonical-module dependency. These are
relocations/adaptations, not reasons to copy duplicate tests or broaden APIs.
The sole core/lib addition belongs to #219; original CSS additions split into
the rail-popover pointer guard in #273 and event-child drag targeting in #259.
The feature selector and unadopted constitution proposal remain intentionally
preserved as recorded under Original commit accounting.

Next collect15935 (#281),31756 (catalog prerequisites),37940 (#271),31708 (#268)
and current-head Nixbot295 without restarting live checks. Finish remaining
shared-hunk/cache-deletion/specification reconciliation and #269's intermittent
fixture diagnosis; do not mark complete or propagate unverified corrections.

### Retained client decisions and shared-file review — 2026-10-07

The preceding estimate-only answer made no goal progress. This iteration resumed
the four exact live check handles, reconfirmed #216 merged and verified zero
tracked difference between the original worktree and saved snapshot d364270a.
Its 18 dirty paths remain unchanged. No original worktree was edited.

Read the installed Spec Kit analyze procedure and ran its prerequisite command
with `--json --require-tasks --include-tasks` in the original worktree. It resolved
feature 015; no extension hooks exist. The task file explicitly says the full
feature breakdown is incomplete. Consequently this is a focused preservation
review, not a completed full-feature Analyze or permission-implementation gate.
No analyzed specification or task checkbox was changed.

Reviewed the seven dirty specification deltas against their saved snapshot,
the extracted client contract and #248's historical extraction context. FR-035
and FR-036 remain consistent across the source spec, plan, operation matrix,
dependent-spec register and T238: global client defaults need global rate plus
Client authority; archive requires archived projects, restore changes only the
client. The compatibility reader does not implement either write contract.
The six named registered-session cases map T237 to client scope, field exclusion,
tenant/filter/count behavior, strict policy, activity and revocation. These are
subsets of FR-006/007/008/010/018 and SC-002/003, not completion of those broad
requirements. T238 maps FR-035/036 but is retained unfinished work, not an
executable full-workflow acceptance package. No new product decision is needed
to preserve these artifacts, and no spec regeneration is warranted.

The original historical progress and quickstart results refer to verifier76298,
not #281's current head. Keep the seven files assigned to the retained follow-up
above, with their approved decisions also published in #281's contract; #248
continues to describe the committed db3935d snapshot. This deliberately separates
unfinished client workflow from the extracted reader rather than duplicating
seven historical documents or claiming the new policy is ready.

Refreshed the existing shared-line triage report (22732, exit0 after retrying a
sandbox process-spawn denial). The recovered report icons and requester matrix
now have matching source ownership. Manual comparison additionally establishes:

- #270's Projects overview keeps the original keyed resource binding and adds
  `initial_client` to preserve master/#216's client-to-project navigation.
- #270's browser fixture authenticates through the same dev-login endpoint and
  verifies its redirect without mounting unrelated legacy Timesheet readers;
  its project authorization assertions remain intact.
- #268's scoped Reports production subtree is byte-identical to the original.
  Its UI test changes only legacy dependency-double signatures (`User` versus
  `CurrentUser`/`UserListItem`, and the legacy tag call); canonical production
  models and assertions remain. Combined signature reconciliation is still a
  cross-PR integration gate, not implied by this standalone comparison.
- #220 inlines the exact original organization's `FOR SHARE` query; #232 tests
  inline the exact `FOR NO KEY UPDATE` query. Both match db::lock_organization.
  #232's previously documented xmin/no-op-row-update adaptation preserves the
  winning-revocation, unchanged-organization and snapshot-retry assertions
  without requiring the separate access-revision schema.
- #259 owns Timesheet navigation messages/tests; #260 owns permission-editor
  messages/tests. Each also retains master's Client editor behavior. Their
  combined navigation dispatcher and browser registrations still need a union
  check; separate passing suites are not that proof.
- `pub(crate)` audit/editor modules retain the binary's internal consumers;
  modified storage doc comments do not remove the registered modules. The two
  relocated HTTP matrices remain explicitly called in #242 and #253.

Live checks remain15935 (#281cd8d1db7),31756 (catalog prerequisite e9f793df),
37940 (#2715beac2e1),31708 (#2681be11e74). Catalog tests finished and deployment
VM checks started; client SQLx passed and its application tests are running;
the two restored-source packages built and are in Clippy. Nixbot295 remains
IN_PROGRESS on #270737aa13a. None is counted as a completed gate here.

Next collect those exact handles, finish cache-deletion and shared-registration
ownership, verify the necessary cross-PR unions, and diagnose #269/ARM from new
evidence. No merges, closures, new functionality or policy activation occurred.

Catalog prerequisite31756 subsequently finished exit0, ending with all checks
passed on unchanged e9f793df7c1dc6d52c549840f8501d21c3df6e38. This closes its
fresh full native gate, including SQLx, tests, browser, deployed recovery and
OIDC. The main deployment script finished in91.90s. This complements #279's
already-recorded final-head66969 pass; it is not ARM or a complete cross-stack
verification. Do not restart31756.

The remaining11 original cache deletions were inspected against their candidate
owners: six in #270, three in #272, one in #273 and one in #269. No complete
whitespace-normalized old query matched the current Rust sources/tests. The
#272 project reader and #273 task reader are byte-identical to the original
replacement modules, using a combined visible/count/page query; the #269 editor
now uses protected-field CASE assignments in place of its obsolete settings
update. This narrows the next cache-reconciliation action but does not substitute
for offline compilation/cache validation after removing descriptors. No cache
file was deleted during this iteration or while its owner's gate was live.

### Final-head client and ARM results — 2026-10-07

Client15935 finished exit0 on unchanged cd8d1db7bcc9f5de1d60feaaff4c337c5ad51fa4:
all native checks passed, including live SQLx, application/component tests,
browser and deployed recovery/OIDC. The deployment script took97.91s.
ARM/Darwin were omitted locally. This replaces the earlier failed compilation
and missing-cache attempts as #281's exact-head native evidence. T238 and the
original broad feature acceptance remain incomplete; source task markers were
not rewritten. Do not restart15935.

Nixbot295 completed SUCCESS at2026-10-07T18:13:09Z on #270737aa13a. Fetched its
successful-attribute page and actual ARM e2e derivation log, retained under
`.scratch/nixbot-295-*`. `checks.aarch64-linux.e2e` executed successfully in21m,
not merely reused a skipped result; its script finished in1226.11s. SIGTERM and
SIGKILL checkpoint/recovery and repeat-import assertions all completed. The
previously failing SIGKILL waiter appeared in71.82s; recovery took80.68s and the
last reimport91.14s elapsed under the existing wait helper. No failure diagnostic
ran. Both architecture formatting and x86 e2e attributes also succeeded;16 other
attributes were already built.

This is current-head ARM green evidence, not a root-cause fix for build266's
intermittent timeout. The diagnostic-only change leaves successful execution
unchanged, and timings remain close to the existing wait threshold. Preserve the
previous failure and do not infer stable timing, widen deadlines or claim that
the added diagnostics repaired runtime behavior. Further runs are warranted by
actual source changes, not repeated polling of completed295.

Ledger deb861d1 and #279's updated composition result were published successfully
(61706 and5920 exit0). Updating #270/#281 descriptions and this result receipt
is the next publication action. Only37940 (#271) and31708 (#268) remain live
from this batch; cache cleanup, cross-stack unions and #269 diagnosis remain.

### Original cache-deletion ownership and replay diagnostics — 2026-10-07

Previous iteration made progress: exact-head client/catalog checks passed and
ARM295 was inspected, not merely polled. #270/#281 descriptions and ledger
8eb51864 were published (53747/84196 exit0). Reconfirmed #216 merged before
this iteration's edits; original branches, dirty source and real data untouched.

Read the existing #269 browser fixture and pinned derivation environment.
Scratch-only instrumentation observes request/response/failure events and route
interception without changing any assertion, input, production source or binary.
85251 ran five repetitions on the exact 7r6v6zz package with a disposable runner
database and exited0. Each retains the two-identical-requests check, unchanged
committed revision, protected-state/history assertions and fixture cleanup.
This fails to reproduce89598, not proof its cause is fixed; no speculative
runtime or assertion change was made. Scratch runner/wrapper are preserved under
`.scratch/editor-replay-diagnostic/`.

The existing combined Reports branch09878591 already contains all five editor
navigation guards (permission, client, invoice, project, Timesheet). Fresh pinned
Node check34646 passed all19 navigation/storage tests. This closes the specific
shared-dispatcher uncertainty noted above, not the full updated cross-stack gate.

Recovered the remaining11 original SQLx descriptor deletions after checking
original deletion commits, replaced queries and current readers. All changes
are cache-only, reversible through Git; no query, assertion or schema changed.

| Owner/head | Original deleted query prefixes | Fresh full native handle |
| --- | --- | --- |
| #26953183702 | a1ea432d7440 | 94084 |
| #2700b4fd421 | 22bd5f392be7,6bd34bcfb204,baf75ef8ba55,bc3f9ea5c9aa,c1f2843a6aaf,e25b27423bd4 | 25727 |
| #2728b8b0c0b | 1f0c682793eb,40899460e93f,4c92eb5086da | 46301 |
| #27365c4aa23 | 3036fe659e73 | 55108 |

All four commits/descriptions are published (39379/19144/3709/18786 exit0);
checks are running and worktrees are frozen. Parent removals have not yet been
propagated into dependent heads. Earlier green evidence remains tied to its
older commits, including Nixbot295 on737aa13a.

Refreshed blob inventory8095 passed. Every one of the20 original deleted cache
paths now has an owner where it is absent: the11 above, #237 (074eb86a0a23 and
4fa0a9d60b98), #228 (9ff1b7d9e9c8 and ed1c72f60026), #249 (548235be5387),
#247 (717614d03bf8), #263 (bcde45c163fc), and #276 (2a22de0eb8e0 and
8aae7affad63). Successful new-head compilation/cache checks are still required;
absence alone is not verification. The1136 exact blobs and58 shared/adapted
paths remain triage classifications, not a completion percentage.

Next collect37940/31708 and the four fresh cache-head gates above. Then integrate
the verified shared/cache deltas into the existing verification compositions,
finish shared-registration ownership and update the final delivery table from
current heads. Do not rerun completed15935/31756/295 or equate repeated isolated
replay success with diagnosis of the old #269 failure. No merges or closures.

### Disk-exhaustion recovery — 2026-10-07

The concurrent build batch exhausted filesystem space. Sandbox startup failed
with ENOSPC; #26831708 and #27137940 are terminal exit1, not live or green.
#268 explicitly failed creating a compiler temporary file; #271's linker failed
with SIGBUS during the same exhaustion. Their browser and live SQLx stages
had completed, but neither whole gate passed.

#27025727 and #26994084 also ended exit1 with disk-exhaustion/dependency errors.
#27246301 and #27355108 reported ENOSPC/compiler-link errors but kept building
independent derivations. Read-only PID/cwd checks identified exactly3126834 and
3126904 in those two worktrees; sent SIGINT only to those task-owned Nix clients.
Both handles are now terminal exit1. No worktree, database, backup or cache was
manually deleted. Nix released temporary build state and available space rose
to38GB. None of these failures establishes a missing SQLx descriptor or a
functional regression, and none is counted as verification.

Ledger fe0b2a1d reached GitHub, confirmed by ls-remote, despite a local tracking-ref
write warning during exhaustion. The worktree remains clean before this receipt.
All six old handles above are closed: do not poll or restart them concurrently.
Retry unchanged #268 first with `nix flake check -L --max-jobs 1 --cores 2`;
then process the remaining exact heads one at a time, reusing completed outputs.
Future whole gates must use this bounded queue rather than another parallel
compilation batch. New68739 is the sole live local full check, on unchanged
#2681be11e74. Queue after it: #2715beac2e1, #26953183702, #2700b4fd421,
#2728b8b0c0b, #27365c4aa23. This queue is not a background scheduler; start each
only after the preceding handle is authoritatively terminal and space is checked.

### Current-head CI evidence and distinct failure classes — 2026-10-07

The preceding estimate response was status-only, not goal progress. This
iteration reconfirmed #216 merged and polled68739 directly: still live,189 core
tests passed, app tests executing. No second local full build was started;
available space is85GB. No original source, application or assertion changed.

Fresh GitHub snapshot `.scratch/permission-ci-followup.json` ties each check
to its current head. Nixbot successful-attribute lists confirm browser, Clippy,
deployment, OIDC, package, SQLx, tests and formatting for both Linux architectures
on #2715beac2e1 ([296](https://nixbot.numtide.com/repos/github/numtide/horae/builds/296))
and #281cd8d1db7 ([293](https://nixbot.numtide.com/repos/github/numtide/horae/builds/293)).
Remove #271 from the local retry queue; those exact-head results need not be
repeated. They do not certify wider integration or a different future head.

Inspected failure logs rather than treating all red checks as infrastructure:

| Build / PR | Observed failure | Next diagnostic boundary |
| --- | --- | --- |
| 297 / #268 | ARM deployment reached SIGKILL setup, then timed out93.94s waiting for the500-row advisory-lock waiter (90s bound) | No job-state diagnostics in that head; cannot distinguish slow processing from failed work from this log alone |
| 286 / #279 | Build page identifies ARM deployment as its sole failed attribute; timeout94.96s | Not an unexplained aggregate failure; inspect detailed boundary before remediation |
| 238 / #219 | Native CSV cancellation regression retried while the old import lock was still busy;818 passed,1 failed | CSV test assumes body closure implies PostgreSQL lock release; equivalent API cancellation test already waits for typed Busy within5s |
| 233 / #236 | Three native CSV recovery tests exceeded the first-checkpoint wait;859 passed,3 failed | Check scheduling/resource contention and checkpoint progress; do not remove assertions or blindly lengthen waits |
| 234 / #224 | Two native CSV recovery tests exceeded checkpoint/recovery waits;820 passed,2 failed | Distinct from238's cancellation race; no claim that one change fixes both |
| 260 / #243 | ARM CLI authorization matrix returned indeterminate_submission exit6 instead of expected1;925 passed,1 failed | Inspect transport/server response before classifying as authorization regression or load |

Raw logs and HTML are retained as `.scratch/nixbot-{build}-*.log/html`.
Source tracing confirms CSV parser cancellation and PostgreSQL connection
shutdown are separate events. `release_import` awaits unlock on ordinary exits;
an aborted future instead relies on connection drop. This explains why a
single immediate retry is not a reliable synchronization barrier, but no
test/runtime repair is claimed or applied in this iteration.

Next: collect68739; reuse completed remote gates for the remaining cache heads
before starting any queued local retry. Resolve the observed failure classes
without changing product behavior, then propagate verified shared/cache deltas
and finish registration/composition conservation. PRs remain drafts; no merges.

### CSV cancellation synchronization and recovered native gates — 2026-10-07

Previous iteration made progress: exact-head successes and distinct failure
logs changed the verification queue; ledgerfc86f304 and #271/#281 descriptions
were published. Reconfirmed #216 merged before this iteration's change.

68739 finished exit0 on #2681be11e74: complete native flake checks, including
deployed recovery (98.29s) and OIDC. Nixbot297's ARM failure remains open.
The #268 description now distinguishes these results instead of claiming a
running disk-exhausted31708.

Nixbot successful-attribute pages301–304 prove the exact-head results in the
table above. #270/#272/#273 need no duplicate local native reruns. In301,
the missing x86 test attribute is a real failure:1044 passed,1 failed when
`durable_csv_preview_resumes_after_a_crash_without_recounting_rows` retried
while the cancelled PostgreSQL session still held its import lock. This shares
the synchronization defect seen in238's non-durable CSV cancellation test.

Published #2242afe25e3 changes only `engine_tests/csv_streaming.rs` (25 added,
4 removed lines). Both aborted-CSV paths observe the actual import lock before
one real import retry. The helper waits only on typed Busy and fails on any
other error. Callers bound the barrier by5s; the existing cancellation test
keeps barrier plus retry inside its original5s. No checkpoint timeout, original
assertion, production behavior, SQL macro or migration was changed. Review
confirmed the probe uses a separate connection and cannot roll back the old
session or manufacture a successful data import.

Initial narrow98017 and final narrow46685 each exited0:55 engine tests passed,
4 pre-existing manual scale tests ignored,774 unrelated tests filtered. Final
run includes both corrected paths and the three original cleanup regressions.
Cargo formatting86352 and whitespace passed. Full native71277 is now the sole
local full check, bounded to one job/two cores; this worktree is frozen while
it runs. These narrow results do not replace its pending full verification or
claim to fix unrelated checkpoint-timeout/ARM/CLI failures. #224 push57342 and
updated descriptions for #224/#268/#269/#270/#272/#273 succeeded.

Read-only `merge-tree` preflight for Reports09878591 plus catalog9fcc0a0 found
four shared-file conflicts: snapshot visibility, HTTP registrations, modal
resource names and browser suite registrations. Preserve public(crate)
snapshot access, both registration sets, `get_project_overview` for Projects
and `load_timesheet_page` for Timesheet. No composition branch was modified.
Original #212 tracked state still matches its saved snapshot exactly; its18
dirty paths remain present. No GitHub merges or closures.

Next: collect71277 and ARM302–304 results; propagate the verified #224 barrier
and cache deletions into the existing dependency/composition chain without
dropping registrations. Resolve remaining distinct CI failures, then complete
the cross-stack conservation review and current-head delivery inventory.

### Cross-stack verification composition — 2026-10-07

The intervening estimate-only response was not progress. This iteration
reconfirmed #216 merged and continued the unfinished local composition in
`.worktrees/permission-split-integration`, branch
`integration/permission-split-check`. No delivery branch was rewritten, pushed,
merged or closed on GitHub.

Local composition af7c35dc includes Reports09878591, catalog9fcc0a09 and the
published heads #2681be11e74, #26953183702, #2700b4fd421, #2728b8b0c0b,
#27365c4aa23, #2715beac2e1, #28024c5d0e1, #281cd8d1db7 and #2242afe25e3.
Conflict resolutions preserve snapshot visibility, both HTTP registration sets,
the Projects/Timesheet resource names, all browser suites and both existing
task-contract sections. This branch is a verification fixture, not a proposed
delivery PR or a permission-policy activation.

The reusable composition audit now accepts explicit parent heads. At615119fa
it accounted for988 exact paths,85 shared paths requiring semantic review and
three replaced SQLx entries; no unaccounted single-owner path remained. All34
browser suites and24 HTTP matrices are retained once. The three cache exceptions
require absence of the old entry and exact preservation of its replacing
Projects/time-command source from an owning parent. Inspected removals cover
task activity and link-rate currency, not missing query metadata. These counts
are conservation evidence, not a claim that shared code or the feature is done.

Review found Reports' isolated test doubles still using the standalone branch's
old user types and zero-argument tag reader. Local af7c35dc adjusts only those
three signatures to the composed CurrentUser/UserListItem/requester contracts;
production and assertions are unchanged. Carry this adaptation into #268 when
its dependency base is updated; do not apply the new signature to its old base.

Formatting and shell/JavaScript syntax checks88256/5426 passed; final Rust
formatting77380 passed. Node42847 passed all19 navigation/recovery-storage
regressions. Full composed compilation, SQLx, browser and deployment verification
remain pending; no previous parent result substitutes for them. #224's sole
full local run71277 remains live, with client and server package builds completed.
ARM302 advanced to19/20 attributes done; e2e remains building. No restart was
requested. Disk has98GB available. Original #212 tracked changes still match
the saved snapshot exactly, with its18 dirty paths preserved.

Next: collect71277, then run the full serial gate on af7c35dc (or its explicitly
recorded successor). Finish the shared-file semantic review and publish necessary
integration adaptations on the appropriate dependency PRs after verification.
Collect ARM302's diagnostic result and resolve the distinct remaining CI failures;
reconcile the final delivery inventory without treating this fixture as shipped.

### ARM recovery evidence and bounded VM budget — 2026-10-07

Previous iteration made progress: composed source af7c35dc and ledger099b43ab
were committed, with the ledger published. This iteration collected terminal
results rather than restarting the confirmed-live local71277.

Nixbot302 on #2700b4fd421 failed only ARM e2e. Its raw log is retained at
`.scratch/nixbot-302-arm-e2e.log`. QEMU reports unavailable KVM. Both interruption
scenarios succeeded: SIGTERM lock waiter66.67s/recovery73.57s, SIGKILL lock
waiter84.82s/recovery96.34s elapsed. The final repeated import exceeded the90s
wait (93.26s elapsed). Diagnostics show503 processed items, one running attempt,
future lease, no last error, an active backend with no blocking PIDs and only
the granted import advisory lock. The preceding three jobs succeeded with1003
processed items. This supports an insufficient TCG workload budget, not a claim
of a deadlock, cancellation defect or proven completion of the fourth job.

Published #270b480f9d2 changes only `nix/checks/e2e.nix`: the three import
boundary/completion waits allow300s on ARM and remain90s on x86. Gate acquisition
stays90s; shutdown and diagnostics stay45s/15s. Input size,500-row interruption,
exact1000-row results,60000minutes, reports and two-attempt recovery assertions
are unchanged. No production timeout, lease, retry or skipped test was added.
The existing blueprint check remains registered on both Linux architectures.

Formatting and whitespace passed. Focused41517 tests the actual evaluated
scripts for both architectures: six success/failure/diagnostic-error cases,
90s default and300s explicit bounds, and all three budgeted call sites. An initial
attribute lookup and dev-shell Python invocation failed before testing; corrected
commands use `e2e.config.testScript` and Python from the pinned nixpkgs input.
Current-head full CI is pending, not replaced by these focused tests. PR270's
description records the evidence and remains draft. The local-only composition
now includes this commit at5c43f24a; no composition full gate has started.

Nixbot303 (#2728b8b0c0b) and304 (#27365c4aa23) finished successfully. Their
successful-attribute pages explicitly include both Linux architectures' browser,
Clippy, e2e, OIDC, package, SQLx, tests and formatting. Reuse these exact-head
results; do not rerun the old disk-exhausted native checks. Cross-stack semantic
review remains distinct and unfinished. Local71277 on #224 is still live: both
package builds and Clippy passed; the browser suite is running.

Next: collect #270's fresh ARM result and #22471277; once the sole local full
check terminates, run the serial full gate on composition5c43f24a. Continue the
shared-file semantic review, then propagate verified repairs into dependent PRs
and finish the current-head inventory. No GitHub merges or closures.

### Focused composition review — 2026-10-07

Previous iteration made progress through the published ARM repair and terminal
CI evidence. Composition5c43f24a is clean; its crate sources are byte-identical
to af7c35dc (only the recorded ARM test repair followed). The current-head
composition audit passes with988 exact paths,66 shared paths equal to an input,
19 genuinely combined paths and the three inspected obsolete-query replacements.
All34 browser suite names and24 HTTP matrices remain registered once.

Focused self-review of those19 combined paths checked both parent diffs, not
just the auto-merge result:

| Boundary | Preserved behavior / review result |
| --- | --- |
| Route, Admin shell, sidebar and shell fixture | Tasks/People destinations coexist with Timesheet's selected-person parameter; return links select own time; old person-less route tests and malformed-person rejection remain |
| Reports route and isolated fixture | Access gate/requester binding remains from Reports; legacy tags use the Project reader's optional requester signature; af7c35dc is the previously recorded test-double-only adaptation |
| Harvest router and pagination registration | Client list/count/direct-ID delegate to #281's same-transaction reader, alongside the unchanged Project/Task readers; all three regression modules remain registered |
| Server module exports and HTTP registration | ProjectReadAccess, report permission-state exports and crate-visible snapshot coexist; all delegated-time, export and task matrices retained |
| Browser waits, suite runner and New Project | Projects wait for overview, Timesheet for page loading; reset checks exact pre-filter project identities rather than assuming seed-only rows; all existing assertions retained |
| CSS and icons | Only the two existing narrow pointer-event fixes are combined; Tasks glyph and both Reports chevrons coexist; no token or global selector rewrite |
| CSV cancellation fixture | Exactly #224's bounded session-release synchronization over the task-lifecycle fixture; checkpoint, rollback, count and retry assertions unchanged |
| Task contract | Existing link/catalog/atomic-rate sections and project-editor lifecycle section retained; no new requirement or completed-parity claim |

Additionally, invoice cross-consumer fixtures preserve exact amounts (now
Some(amount), not a zero fallback), maximum-integer overflow and void behavior.
Project exports/delivery remain byte-identical to #271; task editor controls
remain byte-identical to #280. The additive permission migration still defaults
policy to0. Observed policy-setting statements are test fixtures, including the
preview file's cfg(test) module; no activation command was added by composition.

No additional high/critical integration finding was identified in this bounded
review. This is not an independent review of all inherited implementation, nor
proof of compiled/runtime compatibility. Browser-script syntax plus shell
syntax91401 passed. Full composition compilation, SQLx, tests, actual browser
and deployment gates remain required. Local71277 is still live in the existing
browser suite; its worktree remains frozen and no second full build was started.

Next: collect71277 and start the serial full composition gate when it terminates;
collect ARM311 on #270b480f9d2. Reconcile the remaining inherited-file/original
change inventory and dependency delivery after those results, retaining explicit
unfinished client work and without merging any GitHub PR.

### Session cleanup gates complete; combined gate started — 2026-10-07

The preceding user-facing estimate was no progress. This continuation re-read
the objective and repository rules, reconfirmed #216's merged commit, polled the
existing local check rather than restarting it, and collected terminal evidence.

#224 remains at2afe25e3. Local71277 completed with exit0 and `all checks passed!`
for the bounded `nix flake check -L --max-jobs 1 --cores 2`. This local result is
x86_64-linux only; the output explicitly excludes incompatible architectures.
[Nixbot308](https://nixbot.numtide.com/repos/github/numtide/horae/builds/308)
also succeeded on this head. Its successful-attributes page explicitly lists
browser, Clippy, e2e, OIDC, package, SQLx, tests and formatting for both Linux
architectures. The pages are preserved as `.scratch/nixbot-308-final.html` and
`.scratch/nixbot-308-final-succeeded.html`. GitHub CI37670586943 passed Flake Check
and Format. The PR description now records these current-head results instead
of calling them pending. The draft remains while delivery reconciliation and
cross-stack verification are unfinished; these results do not certify other
heads or resolve the distinct checkpoint-timeout evidence by inference.

After71277 terminated, clean composition5c43f24a was checked and its full serial
gate started as74548 with the same bounded command. Output is retained in
`.scratch/permission-split-full-5c43f24a.log`. Its worktree is frozen during the
check. This is the only running local full gate;102GB were available before
launch. No GitHub merge, original-source edit, real-data change or permission
activation occurred. ARM311 on #270b480f9d2 remains in progress; no rerun was
requested and its timeout repair is not yet declared verified.

Next: collect74548 and311, investigate any new composed-boundary failure before
propagation, then reconcile dependent PR bases and carry the verified shared
repairs into their delivery chains. Preserve the original backups and the
explicitly unfinished client work. Do not use standalone passes as proof that
the combined stack passed.

### Complete delivery composition and ARM acceptance — 2026-10-07

Previous iteration made progress by collecting #224's complete gates and starting
the next serial run. Local74548 remains live on unchanged5c43f24a: client build
passed and the server package build is ongoing. Do not edit that worktree or
start a competing full gate.

Comparison against all58 extraction heads exposed the scope limit of5c43f24a:
it combines Timesheet/Reports/Projects/Tasks but does not contain12 current heads.
Those are preserved, not lost. #246 includes the missing #235/#236/#237 chain;
the nine additional roots are #221/#222/#225/#233/#238/#239/#242/#246/#248.
#221's original content was already present through other ancestry; #222 adds
its current database-requirements note. A passing5c43f24a cannot certify these
remaining boundaries.

Prepared a separate local-only worktree `.worktrees/permission-delivery-complete`,
branch `integration/permission-delivery-complete-check`, at
`efae752272402457da6393791048b4e3c0a68d5f`. It contains all58 exact heads from the
delivery snapshot, without rewriting their branches or creating a combined PR.
No full build has started there. The two substantive conflict resolutions retain:

- #242's invoice-identity HTTP matrix alongside all24 existing matrices, once
  each; no replaced or skipped assertion.
- Every permission module/test, including preflight, and the CSV batch boundary
  that reads the next batch before opening its organization-locked transaction.
  Plain `connection.begin()` must not replace `begin_import_transaction` there.

The upload implementation, approval implementation/isolation tests, import
commands/requester tests, invoice authority tests and preflight implementation/
tests are byte-identical to the original db3935db. HTTP upload/download revocation,
requester-spoofing and error-response assertions are present again in the complete
composition. CSV cancellation synchronization remains exactly #224's recorded
follow-up over the original fixture. Rustfmt reordered one module declaration;
that ordering is committed and subsequent formatting70916 passed without changes.

The existing conservation audit was run against all58 inputs, not just the
subset's leaves. Its first run identified four explicit adaptations. Inspection
confirmed two intermediate legacy CSV descriptors replaced by the scoped cursor,
and one intermediate task-link descriptor replaced by lifecycle/rate-currency
validation. Each accepted deletion requires absence plus the exact corresponding
source blob from a parent that also lacks the descriptor. The Reports fixture
requires the exact previously reviewed af7c35dc blob and its ancestry; no arbitrary
fixture difference is accepted. That fixture differs from the original only by
the unused parameter spelling `_expected` versus `_`.

Final audit72279 passed:1157 exact paths,108 shared paths (80 equal to an input,
28 composed), six inspected query replacements, one exact fixture adaptation,
zero unaccounted single-owner paths,34 unique browser suites and25 unique HTTP
matrices. Evidence: `.scratch/permission-delivery-complete-audit.json`; the former
subset report is preserved as `.scratch/permission-split-composition-5c43f24a.json`.
These checks prove ancestry/preservation constraints, not semantic or runtime
acceptance of all28 combined paths. No completed-parity claim is made.

[Nixbot311](https://nixbot.numtide.com/repos/github/numtide/horae/builds/311)
passed on #270b480f9d2. Both Linux e2e/formatting attributes executed successfully
(ARM e2e19m9s);16 unchanged derivations were already built, including browser,
Clippy, SQLx, tests, package and OIDC on both Linux architectures. Attribute pages
are retained in `.scratch/nixbot-311-final-succeeded.html` and
`.scratch/nixbot-311-final-cached.html`. This is current-head acceptance of the
bounded ARM repair, not proof that it resolves every historical failure class.

Next: collect74548, then run the full serial gate on complete compositionefae7522
(or a documented reviewed successor). Finish its changed-boundary review before
propagating the now-verified #224/#270 repairs and refreshing dependent PR bases.
No GitHub merges/closures, real-data changes or runtime policy activation.

### Shared CI prerequisite extracted — 2026-10-07

Previous iteration made progress through the complete local composition and ARM
acceptance. Local74548 is confirmed live on frozen5c43f24a; both release package
targets completed and the server all-target Clippy check is running. No second
full local gate was launched.

Opened draft [#282](https://github.com/numtide/horae/pull/282) over unchanged
origin/master8b3cc257: branch `test/import-recovery-checks`, worktree
`.worktrees/import-recovery-checks`, head `e39f033a12bf3a708b54472fd7a80fb8672bb5df`.
It isolates the test repairs needed by the extraction stack, rather than making
CI depend on Project readers or production session cleanup. Two files change:
CSV engine fixture and NixOS recovery script;58 additions/11 removals. No runtime
code, schema, new dependency or product behavior is included.

| Source patch | Independent commit | Stable patch ID |
| --- | --- | --- |
| 737aa13a VM failure diagnostics | c5ea7b0a | 29757062ad506e3fc0c6976e198220c13b751b40 |
| b480f9d2 ARM workload bound | 00b0742d | f18c2f514cd5a25c589604b81bf54e56d8380f91 |
| 2afe25e3 cancellation fixture barrier | e39f033a | 9b983651968f82a97483988966201a5beb016a8a |

All patch IDs match their source. The entire changed CSV file is byte-identical
to #224's current version; the entire Nix file equals #270's current version.
Focused14969 passes formatting, both architecture evaluations and the existing
six deadline/diagnostic cases for each script. Fresh standalone CI remains
required: source-head Nixbot308/311 are supporting evidence, not this PR's gates.
Publication16559 succeeded. #224/#270 remain preserved at their prior heads;
the eventual dependency refresh must remove these duplicate review deltas.

Additional bounded review of compositionefae7522 confirms its Harvest callback,
credential authorization, account switching, import commands, engine fixture
setup, report-migration fixture and user-role helper match the original source.
The callback authority check now resides inside the reserved transaction, as in
the original #235 extraction; no pre-exchange-only guard was substituted.
All `week_total_minutes` callers supply the organization identity required by
#239. Invoice authority tests remain registered alongside the existing suites;
the read-side additions are the separately retained #220/master snapshot work.
No new high/critical finding in these reviewed boundaries; this is not a claim
that all pending CI failure classes or the full semantic audit are resolved.

Next: collect74548 and fresh #282 CI. After the sole local full gate ends, verify
complete compositionefae7522. Use #282 as the bounded shared prerequisite when
refreshing dependent branches, preserving explicit backups and avoiding blind
reruns or counting old heads as new-head verification. No GitHub merges.

### First delivery rebased onto shared CI — 2026-10-07

Previous iteration made progress by publishing #282 and its preservation proof.
Reconfirmed #216 merged and #224's clean worktree/remote2afe25e3 before rewriting
the extraction branch. Saved `backup/import-session-before-shared-ci-20261007`
and verified complete-history bundle `.scratch/pr224-before-shared-ci-20261007.bundle`
(SHA-256 `cc723189e3d45770a385df47728c3c9e0e729c1bb11e1ec41124d63ca6484885`).
Rebase explicitly disabled updateRefs and signing; original #212/#217 and all
integration worktrees remain unchanged.

#224 is now `c374710a19968afd711113a726faec0a2f150ddb`, based on #282e39f033a.
Only the duplicate2afe25e3 patch was dropped as already present upstream. Its
review delta contains one production/cleanup-test commit: the exact8ffac9de patch
with stable ID `633e655c8c4207c69e84cb13e34c2c9a27d69f99`. All files match the
previous2afe25e3 tree except `nix/checks/e2e.nix`, which matches #282 exactly.
Formatting36405 and whitespace checks passed. Retarget/publication57738 succeeded
using an explicit lease for the old remote SHA; GitHub confirms the new base,
head and draft state. Nixbot319 has begun evaluating this head. Previous308/71277
passes remain labeled historical, not counted as c374710a verification.

Current #282 Nixbot317 remains live. Local74548 remains live on frozen5c43f24a;
Clippy completed and the browser suites are running. The complete local
compositionefae7522 still contains the old #224 ancestry but identical Rust/
test content and the same final Nix script; record tree/patch equivalence when
reconciling its inputs, rather than claiming new-head ancestry.

Next: collect the sole local gate and #282/#224 CI, then verify complete
compositionefae7522. Continue the dependency refresh from its shared roots,
retaining backups and proving expected trees before publishing. Do not modify
the active-check worktree, start competing full builds or merge GitHub PRs.

### Shared-root refresh — 2026-10-07

Previous iteration made progress with #224's verified rebase. Updated the next
three clean, single-commit roots onto #282e39f033a, with updateRefs disabled:

| PR | Previous head | Published head | Functional patch |
| --- | --- | --- | --- |
| #219 | 94a5d606 | a936c129019bd7b90b7adb2b90ab34288f36a5c3 | Unchanged |
| #220 | ae152748 | c2f587d6d3cf2bf52ccc37ef139dd1863fe0406e | Unchanged |
| #223 | 1dc4b6bc | bc74af8d198704fe033f50353b08b2d0dec0dc29 | Unchanged |

For each, range-diff reports an identical patch, its resulting tree equals the
precomputed composition of old head and #282, all other files are byte-identical
and the two CI files exactly equal #282. No manual conflict resolution or code
rewrite was needed. Formatting21531/73103/50969 passed without changes.

Recovery refs are under `backup/shared-ci-roots-20261007/`, using the three
worktree basenames. Their complete-history bundle
`.scratch/shared-ci-roots-20261007.bundle` verifies, SHA-256
`2aea5850cefd26c7e98f19aa77498314e9d212dcdc00cd40dc03b8cbb0f17238`.
Publication43969 used one atomic push with explicit old-SHA leases. All three
PRs now target `test/import-recovery-checks`, remain draft, and describe earlier
test results as historical. GitHub confirms each published head/base.

Refreshed inventory62744 includes all59 deliveries and finds no local/remote
head mismatch. It also explicitly identifies the newly stale direct children
#221/#222/#232; their transitive composition bases are not yet reconciled.
Do not treat those children as containing the shared repairs. Original-work
conservation reports retain their recorded old input SHAs; the new snapshot is
not an input-ancestry proof for the earlier local compositions.

Local74548 remains live on frozen5c43f24a. Both builds, Clippy and browser checks
completed; live-schema SQLx preparation is running. #282 CI remains pending.
Next: collect74548, then verify complete compositionefae7522; continue the
dependency-ordered refresh through #221/#222/#232 with the same preservation
checks. No additional local full gate, GitHub merge or original-source edit.

### Shared-child refresh — 2026-10-07

The preceding user-facing estimate was status-only, not goal progress. Revalidated
the live local check via session74548 and confirmed #216 remains merged before
continuing. This iteration updates three clean children without changing their
functional patches:

| PR | Previous head | Published head | Current parent |
| --- | --- | --- | --- |
| #221 | 7866b97f | cc474e87befc9a8769268c0358db7f9b2773620b | #219 a936c129 |
| #222 | 23f258b7 | 1fef1a2461052a6fd43087c50812bafaabffd6a0 | #219 a936c129 |
| #232 | 6bf2fbcf | c90aa898a40175661a3493c429be0ca06bf64d51 | #220 c2f587d6 |

All four replayed commits have identical range-diff patches. Each resulting tree
equals its precomputed composition with the exact previous parent as merge base.
Automatic merge-base selection initially reported an add/add conflict because
the parent itself had been rewritten; that was a read-only preflight, not a
worktree conflict or code change. Explicit old-parent selection and all rebases
then succeeded without manual resolution. Every old/new file is byte-identical
except the two inherited CI files, which exactly match #282. updateRefs and
commit signing were disabled for each rebase.

Recovery refs are under `backup/shared-ci-children-20261007/`, named after the
three worktrees. The complete-history bundle
`.scratch/shared-ci-children-20261007.bundle` verifies, SHA-256
`0841df8573ae42722c615b737b8aef42776df68e96013ccfd6e7fa404da7e118`.
Formatting21009/14903/86144 passed without changes; whitespace checks passed.
Atomic publication90927 used exact previous-head leases. GitHub confirms all
three heads, unchanged parent branch names and draft status. PR descriptions
explicitly distinguish historical full gates from pending fresh-head checks.

Inventory94258 finds59 drafts, no remote/local mismatches,15 successful heads,
36 failed heads,1 running aggregate and7 new heads awaiting an aggregate.
Only five direct bases are stale; named integration bases still require input
reconciliation, so this does not certify transitive freshness. No original-source
edit, real-data change, policy activation, GitHub merge or closure occurred.

Local74548 remains live on frozen5c43f24a, now compiling the server test suite
after SQLx preparation. Its eventual result covers the recorded subset only.
Next: collect that result and then run the sole full gate on complete composition
efae7522. Continue the dependency refresh with #226 and the remaining independent
roots before reconciling their composition bases. Keep original composition audit
inputs immutable and do not treat earlier green heads as current-head evidence.

### Remaining independent roots and template refresh — 2026-10-07

The preceding iteration made progress by publishing #221/#222/#232 and the
single ledger. Confirmed #216 is still merged and refreshed six more clean
worktrees, reusing all original functional commits:

| PR | Previous head | Published head | Current parent |
| --- | --- | --- | --- |
| #225 | ac723cb6 | 34a32928b6f6076205aabe68b9fc09924148f895 | #282 e39f033a |
| #226 | 5a617c68 | 1f9d20e88eb1b4805d49edf03c4c6ec6ff80d596 | #222 1fef1a24 |
| #227 | 25d75c4e | 8f3a7e640f4d871267104e39e3aebe135992de1e | #282 e39f033a |
| #231 | 2e4bbf9d | 110b365ae894871fac59e24f6d58a2693e905057 | #282 e39f033a |
| #238 | fbaf7a43 | 24aca30d9218845181b7c6418a435adb763f8dac | #282 e39f033a |
| #239 | 3aab2bfa | f4cea603ff8f7c2edc5ab655adfaa937b108a4f8 | #282 e39f033a |

All eight replayed commits have identical range-diff patches; each resulting
tree equals its precomputed composition with the exact previous parent. No
conflict resolution, new logic, schema edit or policy activation was needed.
updateRefs and signing were disabled. All old/new files are byte-identical
except the inherited CI repairs and #226's six original README lines from #222.
The README exactly matches #222. Both CI files match #282 except #231's CSV
test file, which also retains its own original regression tests and barriers.
An intentionally strict whole-file equality check stopped on that overlap;
inspection and stable patch comparison prove its old-to-new difference is only
the shared cleanup patch `9b983651968f82a97483988966201a5beb016a8a`.
No original test or assertion was replaced to satisfy that check.

Recovery refs are under `backup/shared-ci-roots-second-20261007/`, named after
the six worktrees. The complete-history bundle
`.scratch/shared-ci-roots-second-20261007.bundle` verifies, SHA-256
`67dc8836616bd1e0ec6125a823295c51f2c6cde4b49f6e8d181913deabf5e4f3`.
Formatting97926/35874/36088/14725/94559/56856 passed without changes, as did
whitespace checks. Atomic publication84948 used exact previous-head leases;
GitHub confirms all six heads, bases and draft status. Five independent roots
now target #282; #226 retains #222 as its base. Descriptions label earlier gates
historical and explicitly require fresh full checks.

Inventory66860 finds59 drafts, no local/remote mismatch,13 successful heads,
32 failed heads,4 running aggregates and10 heads without an aggregate yet.
The changed totals reflect new commits awaiting validation, not newly observed
test failures. Direct-base stale entries are #228/#272/#273/#275/#281; named
integration bases still require their own transitive input reconciliation.
#282 remains active in Nixbot317 and GitHub Flake Check37676457227, with Format
passed. Do not count those unfinished checks as green.

Local74548 remains live on frozen5c43f24a and is executing server import tests.
The complete compositionefae7522 is still clean and has not started its full
gate; run it only after74548 is terminal. Disk has80GB available, and no second
local full check was started. Next refresh #228 on #2278f3a7e64, then #235 on
that result. Reconcile the profile, invoice and import prerequisite branches
from the refreshed inputs before updating their children; do not claim their
current old ancestry includes these CI repairs. No GitHub merge or closure.

### Shared prerequisite compositions and child refresh — 2026-10-07

Previous iteration made progress by publishing six roots/children with recoverable
backups. Confirmed #216 remains merged. Refreshed five delivery branches and three
existing review/test compositions; no new delivery PR or behavior was added:

| Branch / PR | Previous head | Published head |
| --- | --- | --- |
| #228 project-access-lock-order | 6c67247e | 951baf9d31de0b936477b54a884f3b0a9b04cbfd |
| #235 harvest-connection-authority | 082fa032 | 7331d30b7fe0f9c9c06ded3f8a5f7852df5bae4e |
| integration/profile-command-prerequisites | a108c514 | b929b0d868931f5fc68725c4c50043a1c47804ee |
| integration/invoice-authority-prerequisites | 99eab225 | 99adead5b51d7194589e12fdbbf0fe1e69090356 |
| integration/import-job-authority-prerequisites | 9f4d52e4 | d8fa7945418510a4532398403f6cafdef8d11b23 |
| #233 invoice-write-authority | c30b8c41 | 26bbda2f9320e885b93eff6532abf8914715158c |
| #234 person-profile-commands | 7afbe764 | 6b524f701a91123082e7cc8ad9191cc63017b160 |
| #236 import-job-authority | 526a8ba9 | 5f0c619c09b8da32a0d1ac5c4c617f1f081a715e |

Each rebase preserves every replayed patch by range-diff and exactly matches the
precomputed tree using its old parent. Signing/updateRefs were disabled; no
manual conflict resolution was required. The only inherited changes are the
two shared CI repairs and, for the profile composition/child, #222's original
README note. The CSV cleanup difference has the exact shared stable patch ID,
including the import composition that also retains #231's own CSV tests.

The profile composition contains the patch-equivalent current #221. The invoice
composition contains the unchanged #220 source patch plus exact #232 patch;
four #220 SQLx descriptors are already supplied by #228, not deleted or missing.
Their blobs match #220 both before and after the composition commit. The import
composition retains current #235 ancestry and exact #231 patch. These are review
compositions, not separate deliverables or merge targets; do not claim ancestry
of cherry-picked delivery heads merely because their patches are equivalent.

Verified complete-history recovery bundles and refs:

| Bundle / ref prefix | SHA-256 |
| --- | --- |
| `.scratch/shared-ci-prerequisites-20261007.bundle` / `backup/shared-ci-prerequisites-20261007/` | ddda18c040d66d1df1d00f629bc896a3740937b3de970c09c47fd4347580e8d6 |
| `.scratch/shared-ci-prerequisite-children-20261007.bundle` / `backup/shared-ci-prerequisite-children-20261007/` | 1bb2a5b90f2b2510da00d0df232ad2993cc21632ddc0ad37171693ef7914b6d0 |

Ref suffixes are worktree basenames. Formatting97470/74784/12673/10286/18274
and40002/71405/98563 passed without changes; whitespace checks passed. Atomic
publication52486 used exact old-head leases for all eight branches. GitHub
confirms the five PR heads/bases/draft states, with fresh CI explicitly pending.
Inventory94966 finds59 drafts, no remote/local mismatch,12 successful heads,
28 failed heads,10 running aggregates and9 awaiting aggregates. The six directly
stale bases are #241/#244/#272/#273/#275/#281; transitive compositions still need
input reconciliation. Remaining master-based code roots #240/#242 also need the
shared CI base, while #248 is the separate documentation delivery.

### Subset full-gate result and changed next action — 2026-10-07

Session74548 is terminal, exit1, at exact subset head5c43f24a. It is no longer a
live wait and must not be repolled or called a pass. Both release builds, Clippy,
browser checks and live-schema SQLx preparation completed before the server suite
failed: **1,409 passed,2 failed,11 existing ignored**,377.86 seconds of test time.
Log: `.scratch/permission-split-full-5c43f24a.log`.

1. `reports::limits::tests::authorization::materialized_exports_retain_authority_and_release_cancelled_reads`
   failed at the final organization `FOR UPDATE NOWAIT` assertion with PostgreSQL
   55P03. The test had already observed revocation and a forbidden subsequent read.
   Inspection finds the scoped Entries path uses `reports/limits/time.rs::begin`,
   whose denied `authorize_current(...).await?` drops its transaction; the legacy
   snapshot helper explicitly awaits rollback on denial. This identifies a
   candidate error-path cleanup race, not yet a reproduced or verified root cause.
   Preserve the original NOWAIT and row-preservation assertions. Trace and reproduce
   the shared scoped-export transaction paths before choosing a production or
   fixture change; the scoped XLSX extraction begins in #263.
1. `server_fns::importers::authorization_tests::job_endpoints_enforce_session_role_and_organization`
   again failed in the rejected CSV CLI case: expected exit1/401-or-403, received
   exit6/`indeterminate_submission`. This matches the earlier ARM260 failure class
   but now occurs on native x86. `Transport::csv` streams the file and maps send
   errors to that category; the current log does not expose the underlying error.
   Do not infer timeout, HTTP status, early-body rejection or a safe retry without
   a focused reproduction. Preserve the authorization assertions and secret-free
   public error handling. No CLI production change has been made.

Used the Rust best-practices/testing/async skills for this bounded diagnosis;
no source file was edited. Complete compositionefae7522 remains clean, untested
by a full gate, and contains these same paths. The previous instruction to launch
it immediately after74548 is superseded: first reproduce and resolve the two
failures, then propagate reviewed fixes and run the complete gate. No local full
gate is currently running. Continue independent dependency refresh while focused
diagnostics run; do not restart the failed whole subset merely hoping for green.
No original-source edit, policy activation, real-data mutation, merge or closure.

### Focused failure reproduction and independent refresh — 2026-10-07

Previous iteration made progress through eight preserved branch refreshes and
the terminal74548 result. Reconfirmed #216 merged. Created isolated local-only
branch `test/integration-failure-diagnostics`, worktree
`.worktrees/integration-failure-diagnostics`, from exact failing5c43f24a. Diagnostic
commitc7e5f739d65f733f236a33fc52608f02fd2013bf retains all original assertions and
adds only test-mode CSV send-error output without its URL, plus reader/cancel
context to the original failing organization-lock assertion. No delivery branch
or original source was instrumented, and this branch has not been pushed.

Scratch `.scratch/focused-integration-repro.nix` reuses the existing Nix test
derivation, dependency artifacts and disposable PostgreSQL setup. It runs only
the two exact failed tests, up to20 independent invocations each, stopping a
case on its first failure while still attempting the other. This is diagnostic
reproduction, not retries to declare a required gate green. It neither changes
registered CI nor certifies the full suite. Session10979 is live, compiling the
app test binary; keep the diagnostic worktree frozen. Log:
`.scratch/focused-integration-repro.log`. No root cause or repair is yet verified.

While that build ran, refreshed four clean branches without functional changes:

| PR | Previous head | Published head | Parent |
| --- | --- | --- | --- |
| #240 | 68a0fad8 | 1066cf1aef919c8fe4f8da01cff4a8df5bbf3f0a | #282 e39f033a |
| #241 | 5feaa950 | f6b867597836a0ca83a347f6fabdced1095c5125 | #228 951baf9d |
| #242 | 0abace6f | 893219a94f2514c69046bbd160cf904a8d514622 | #282 e39f033a |
| #244 | a0b3ee4e | 7972e98705edd34f4d7dde81d2b16fb61bc61765 | #234 6b524f70 |

All11 replayed commits are identical by range-diff; each resulting tree matches
the precomputed old-parent composition. Old/new files are identical except the
two inherited CI files and #244's inherited six-line README note, all exactly
matching their new parent. No manual conflict resolution or source rewrite.
updateRefs/signing disabled. Formatting65889/42079/2939/49225 and whitespace
checks passed without changes. Atomic publication84528 used exact old-head
leases; GitHub confirms all four heads/bases/drafts. #240/#242 now target #282;
the two dependent base names are unchanged. Bodies require fresh CI and retain
the unresolved cross-stack failure caveat.

Recovery refs under `backup/shared-ci-projections-time-settings-20261007/` use
worktree basenames. Complete-history bundle
`.scratch/shared-ci-projections-time-settings-20261007.bundle` verified, SHA-256
`4a92d7b35ea01dc9c6b9c7038acede0b90d1e3900f55c8dc5b37a59eecfeb7fe`.
Inventory83754 finds59 drafts, no local/remote mismatches,14 successful heads,
24 failed heads,14 running aggregates and7 awaiting aggregates. Directly stale
bases are #272/#273/#275/#281; remaining named compositions are not certified
transitively fresh. No code delivery remains directly based on master except
the shared CI prerequisite; #248 remains the separate documentation root.

New exact-head completed evidence, not historical-head substitution:

- #282e39f033a: Nixbot317 SUCCESS. All20 registered attributes cover both Linux
  architectures;16 executed (browser, Clippy, e2e, OIDC, package, SQLx, tests and
  treefmt on each),4 unchanged devshell/formatter derivations were already built.
  GitHub Actions37676457227 also completed Flake Check and Format successfully.
- #224c374710a: Nixbot319 SUCCESS. E2e and treefmt executed on both Linux
  architectures;16 identical derivations were already built, including both
  platforms' browser, Clippy, SQLx and tests. This is current derivation coverage,
  not a claim that every test reran. Draft retained for final reconciliation.

Receipts are `.scratch/nixbot-{317,319}-final.html`, `-final-succeeded.html` and
`-final-cached.html`; each succeeded/cached union contains20 unique attributes.
These passes do not certify the failed broader composition.

Next: collect10979 without restarting it on silence; use its error/context
evidence to choose and verify the smallest correct fix in the owning delivery.
Do not weaken the NOWAIT, authorization or preserved-row assertions. Then carry
reviewed fixes into the complete composition and run its full gate. Independent
refresh can continue through import requester and audit/delegation prerequisite
compositions, retaining exact backups and patch accounting. No full local gate,
policy activation, real-data change, merge or original PR closure occurred.

### Import requester publication and deterministic export diagnosis — 2026-10-07

The intervening user estimate response was status only, not goal progress.
Reconfirmed #216 merged before resuming source changes. Collected publication3368:
#237 is still draft, with head491891603f76cae23c8b78eac4a95282e53c2251 and
base `integration/import-requester-prerequisites` at
970f0c6bb9ee1b6013ed9dc6080aa343460f91e6. Both exact-lease pushes succeeded.
The base replays the same two foundation/storage patches onto current #236 and
restores the six-line database README note from #222; #237's two patches are
unchanged by range-diff. Formatting65372/99499 and whitespace passed. Recovery
refs use `backup/shared-ci-import-requester-20261007/`; verified bundle
`.scratch/shared-ci-import-requester-20261007.bundle` has SHA-256
`f216f9cc3fa29d35e86c5158a0d4f218d8473abc5ccc6f9f9228fe030b18ca02`.

Focused diagnostic10979 finished with exit1. Reports passed20 isolated invocations;
this does not clear the original full-suite failure. CLI failed on invocation14:
the CSV request send reports Hyper `BodyWrite` / OS32 `Broken pipe`, not an
observed HTTP denial or a timeout. The public indeterminate-submission result is
therefore not evidence of an authorization bypass. Keep the original CLI status
assertions and investigate delivery of the early HTTP rejection separately; no
transport fix, automatic retry or dependency update has been made.

Pinned SQLx0.8.6 source confirms that transaction drop queues rollback and pool
cleanup flushes it asynchronously, after the `after_release` callback. The scoped
time export `begin` propagates authorization failure by dropping its transaction,
unlike the legacy snapshot reader's explicit awaited rollback. Added regression
`denied_time_export_releases_authority_before_pool_cleanup` on the isolated
diagnostic branch, commitd112409e. It pauses only deferred pool cleanup, expects
FORBIDDEN for an inactive actor, and checks the organization with the original
NOWAIT query before releasing cleanup. No production path or SQL query changed.

Session63981 runs the regression's red phase through the existing Nix test
derivation and disposable PostgreSQL; log `.scratch/export-denial-repro-red.log`.
The diagnostic worktree is frozen while it runs. Next: collect that exact run;
if it proves retained authority, await rollback at the owning transaction boundary
and verify the regression plus relevant export tests before porting the fix to
the owning extraction. No complete-composition gate, merge, closure or real-data
mutation occurred. The full goal remains incomplete.

Regression63981 finished exit1 as expected on diagnosticd112409e: the new test
reproduced SQLSTATE55P03 on `organizations` while deferred cleanup was paused.
The exact red derivation is
`/nix/store/71gq345klrcw3zvras6fw59cn4qdryji-horae-export-denial-repro-0.1.0.drv`.
This establishes the error-path cleanup race independently of timing under load.

Diagnostic fixefd83523198befcfd32efdc29cd572da2a5c11fd explicitly awaits rollback
when shared time-export authorization fails, preserving the original error if
rollback succeeds and the existing database-error mapping otherwise. The change
is eight lines added/one removed; no changed SQL, cache, migration, grants or
success-path lifetime. Formatting84897 and whitespace passed. Regression source
and diagnostic instrumentation remain local-only, not a delivery PR.

Session23364 is the focused green-phase check at that exact head; it runs the
whole `reports::limits::tests::` group through disposable Nix PostgreSQL, including
the new deterministic regression and original cancellation/revocation tests.
Log `.scratch/export-denial-check-green.log`. Do not edit the diagnostic worktree
until this session is terminal or count this as a full-composition check.

Verified the owning draft #263 and its worktree are unchanged at
15bda5cccb84aa405270d208dd10c06faab8a0ef. Its authorization is still inline;
#264 already extracts the identical `authorize_current` helper. Porting the fix
must retain the cleanup boundary in #263 and preserve #264's helper reuse when
refreshing descendants. Next: collect23364, review and port a passing fix with
the regression to #263, then verify that extraction and reconcile descendants.
The independent CLI broken-pipe failure remains unresolved; no assertion has
been weakened and no production transport change was made.

Focused check23364 completed exit0 at diagnostic efd83523:54 passed,0 failed,
1 existing ignored manual export measurement,1368 filtered out. Both the original
`materialized_exports_retain_authority_and_release_cancelled_reads` and the new
deterministic denial regression passed. Exact derivation:
`/nix/store/58hcnhs8fnk7m9vxjrv0y9i9d9hglrjl-horae-export-denial-check-0.1.0.drv`.
This covers bounded export tests, not the whole application or combined gate.

Ported the fix and byte-identical regression to #263 locally, commit
34341049883d8d5752272e963445dc39f3f478d4. The extracted authorization helper is
byte-identical to the one already owned by #264 and tested in the composition;
moving it earlier lets the transaction owner await rollback without duplicating
cleanup across denial branches. Verified helper, `begin`, and regression bodies
against the passing diagnostic source. Queries are unchanged. Formatting4470
and whitespace passed. Reviewed success-path lock lifetime, error propagation,
tenant/actor checks and rollback cancellation; no grant or policy change.

Recovery ref `backup/export-denial-fix-20261007/scoped-time-xlsx` and complete
verified bundle `.scratch/scoped-time-xlsx-before-denial-fix-20261007.bundle`
preserve15bda5cc; SHA-256
`40c98d79d9190dc20c2b88c65652040eb54ece6bc11f887bd5db14304b5624d0`.
Session70597 completed exit0 on #263's own dependency context:43 passed,0 failed,
1 existing ignored manual measurement,1032 filtered out. Both the deterministic
regression and the original cancellation/authority test passed. Log:
`.scratch/scoped-time-xlsx-denial-check.log`; exact derivation:
`/nix/store/04sikfszyngycbjlsrkh7vpkx22w7gp3-horae-scoped-time-xlsx-denial-check-0.1.0.drv`.
Publication49141 completed: GitHub263 confirms34341049, still draft, on review
base83967167. Its description distinguishes current focused checks from older
full gates, documents the cleanup fix and preserves original provenance.

Next: reconcile this cleanup into dependent extractions and the complete
composition, retain it while refreshing #264's previously extracted helper,
and resolve the outstanding CSV CLI transport failure before the complete gate.
No local diagnostic/full gate is running. The diagnostic branch remains local
and retains only its instrumentation plus this fix/regression; do not deliver
the instrumentation. Originals #212/#217/#208 remain untouched. No merge,
closure, policy activation or real-data mutation occurred. Goal incomplete.

### Complete-composition cleanup and isolated CSV rejection — 2026-10-07

Previous iteration made progress by reproducing and fixing export denial cleanup,
verifying both dependency contexts and publishing #263. Reconfirmed #216 merged.
Carried only the regression/fix commits into the clean local complete composition:
efae7522 ->843c8bad ->b775c14c3b360cb39d806b7d28e6b459f0987dab. Both commits are
identical by range-diff to d112409e/efd83523; only the time-export reader and its
regression changed. The CLI diagnostic logging and the original failure-context
instrumentation were not carried. Backup ref
`backup/export-denial-fix-20261007/permission-delivery-complete` preserves efae7522.
Formatting23688 and whitespace passed. This branch remains local-only and has
not received a full gate; the older conservation audit remains historical with
these two explicitly owned #263 changes as an addendum.

Added a focused CLI regression on the local diagnostic branch at b40f52f2:
`small_csv_upload_preserves_immediate_authorization_rejections` sends a small
regular CSV to local HTTP handlers returning401/403 without consuming its body.
It checks each response, up to256 requests per status, failing immediately on a
lost status. No retries of failed assertions, real imports or production transport
changes. The check separates the HTTP upload from sessions, database and job
state. Formatting41790 corrected the test layout; the follow-up49110 passed.

Session18465 runs this regression using the existing Nix test derivation;
log `.scratch/csv-rejection-repro-red.log`. Keep the diagnostic worktree frozen
until terminal. The earlier full endpoint failure remains authoritative even if
this narrower regression does not reproduce it. Investigated the pinned Hyper
early-body-rejection paths and upstream
[issue2384](https://github.com/hyperium/hyper/issues/2384); the issue concerns
similar connection closure but is not proof of this failure's exact cause or
a justified dependency upgrade. Preserve server rejection-before-body-reading,
the CLI's conservative indeterminate result for actual network failure, and all
original authorization assertions. Next: collect18465 and use that result to
choose the smallest supported transport correction; refresh dependent export
branches without losing the verified cleanup. No full local gate is running.

Regression18465 finished exit1 on b40f52f2: attempt9 lost the401 response with
the same Hyper `BodyWrite` / OS32 `Broken pipe`; the isolated test failed in0.02s
after compilation. Exact red derivation:
`/nix/store/i7pg9vwqkc6lidg3sgh471k22scc1fx2-horae-csv-rejection-repro-0.1.0.drv`.
This reproduces the failure independently of permission storage or job state.

Testing a minimal hypothesis on diagnostic86a7e13d55284c25e15b1f72b7c4eb2ee17fb80e:
declare Content-Length from the already validated open file metadata rather than
use chunked framing. The streaming reader,50MiB limit, authorization and
indeterminate-error mapping remain unchanged. Added byte-integrity checks at1,
65535,65536,65537 and262144 bytes; they compare the received length and every byte.
Formatting88902 and whitespace passed. No production delivery is published yet;
do not infer that this one-header hypothesis fixes the failure before testing.

Session58365 runs all CLI import unit tests, then ten fail-fast invocations of
the unchanged real authorization endpoint test on disposable PostgreSQL.
Log `.scratch/csv-known-length-check.log`. Keep the diagnostic worktree frozen.
The entire sample must pass; a failure is not retried into success. Transport
files otherwise match master/#282 apart from diagnostic-only error logging,
so a successful fix can be delivered independently without pulling in permissions.
GitHub #263 at34341049 has fresh Nixbot350 evaluation SUCCESS and build IN_PROGRESS;
no completion claimed. #282 is still open at e39f033a. Next: collect58365; review
file-length/error semantics before extracting a successful correction, or reject
the hypothesis if the regression still fails.

Check58365 completed exit0 on86a7e13d. All20 CLI import tests passed, including
512 immediate401/403 rejections and all five byte-integrity sizes. All ten
invocations of the original real endpoint authorization test passed, with no
failed invocation retried. Exact derivation:
`/nix/store/d1qrbr1236ml6gjjvrhrfjjzdsm9j7a1-horae-csv-known-length-check-0.1.0.drv`.
This is focused evidence, not the complete application gate. Reviewed credential
handling, bounds, unchanged streaming/error semantics, no automatic retries and
server rejection before body consumption. As before, resubmission requires
identical input; this does not introduce a concurrent-file-mutation guarantee.

Extracted the two byte-equivalent commits onto #282e39f033a in isolated
`.worktrees/csv-upload-framing`, branch `fix/csv-upload-framing`:
08019ef1 and8d83b8534026cfe4a7e72c4d8ed5f06f9d0ddc16. Range-diff shows both equal
to b40f52f2/86a7e13d. Delivery delta is exactly two files: one production header
and63 test lines. Diagnostic logging and all permission changes are excluded.
Formatting68752 and whitespace passed. Publication37300 completed successfully
after SSH tried alternative configured keys; GitHub confirms draft #283 with
that head and #282 as base. No authentication blocker remains.

Session80585 is checking this exact independent branch's CLI tests and real
authorization endpoint using disposable Nix PostgreSQL. Log:
`.scratch/csv-upload-framing-check.log`. Keep its worktree frozen until terminal.
The PR description labels this and required CI pending, distinguishing the wider
composition's passing tests from exact-branch evidence.

Carried the same two commits into complete compositionb775c14c as5a753110 and
d5c348516443e5279c551ed018433a6e58dfbc72. Both are identical by range-diff;
backup `backup/csv-upload-framing-20261007/permission-delivery-complete` preserves
the parent. Formatting62236 and whitespace passed. No instrumentation was copied.
Next: collect80585; if it passes, run the sole complete-composition full gate at
d5c34851, keeping that worktree frozen. Continue independent prerequisite refresh
without dropping either correction, then reconcile current-head CI and provenance.
No merge, closure, real-data mutation or policy activation. Goal incomplete.

### Current-head CSV check, delegation refresh and complete-gate failure

The prior iteration made progress by verifying the independent CSV fix and
publishing prerequisite refreshes. The intervening estimate inspected a newly
terminal Clippy failure; no source changed in that status response. This
continuation re-read the saved goal, AGENTS and constitution and reconfirmed
#216 merged as02f7b58. No original worktree, #208, policy or real data changed.

#283 exact-branch check80585 exited0 on8d83b853:20 CLI tests and the real
authorization endpoint passed (831 other tests filtered out). Log
`.scratch/csv-upload-framing-check.log`, derivation
`/nix/store/bp11y2fr34c65xvg4il5mi0hvbw4d7n0-horae-csv-upload-framing-check-0.1.0.drv`.
Its PR body now distinguishes this evidence from the diagnostic composition.
Nixbot352/#283 and350/#263 were in progress at the last remote check; neither
was a passing gate. Inventory30655 found60 drafts,17 successful aggregates,
21 in progress and22 failures, with no local/remote head mismatch at that time.

Delegation/audit refresh publication38131 succeeded atomically with exact
previous-head leases; draft #243/#245 bodies were updated and remote heads
confirmed. Backups remain at `backup/shared-ci-delegation-audit-20261007/*`
and `.scratch/shared-ci-delegation-audit-20261007.bundle`, SHA-256
`0fa611a16193aa3941c0267da77de5488da3e95af100a6d2a94d68308e55a0d6`.

| Branch | Previous head | Published head |
| --- | --- | --- |
| integration/project-delegation-prerequisites | c19210e8 | f01b6cf4c484c77696e9567f850af8e9ba278681 |
| feat/project-manager-delegation (#243) | dde4a8a9 | 7ff4a5005198ca8520a73fcf619051cbc72f20fc |
| integration/permission-audit-prerequisites | 1654d547 | 2053ebbeba861eb0599f7274a221d41799fa0b31 |
| feat/permission-audit-history (#245) | 7dc76ed3 | 322f62b0b05af8b62209a86f6b02ab3201a3eaed |

All four trees matched explicit-old-parent expected compositions. All four #243
patches and six #245 patches were identical by range-diff. Prerequisite
composition replays are patch-equivalent, not current-parent ancestry: the
assignment SQLx descriptor07580408 already existed in the refreshed parent
with identical blobe0a84e52; the audit base's recovered import-authorization
test and permissions module matched original resolution blobs76dcabba and
2c076899 before staging. No new source conflict decision was introduced.
Every old/new path except the two shared CI files and README was unchanged;
these three changes match their established owners. Formatting20760,16096,
13457 and25054 passed. #283 is not inherited by these branches; required
current-head CI and final integration remain pending.

Complete compositiond5c34851 full gate50229 exited1, collected from its actual
session handle. The web release package built, but Clippy failed before the
whole suite completed. Log `.scratch/permission-delivery-complete-d5c34851.log`;
failed derivation
`/nix/store/y8g40mj9s05j2bnss37n0angn04clani-horae-clippy-0.1.0.drv`.
Errors are unused `PreflightCounts`, `PreflightError` and `preflight::read`.
The reader has only test callers and deliberately no public endpoint/CLI/UI.
Original #212 contains the same reader. Standalone #246 inherits a module-wide
inactive-storage expectation; the composed consumers no longer need that
general annotation, exposing the three remaining diagnostics. This is not
evidence that preflight or policy activation is implemented as a public feature.

### Preflight prerequisite refresh and localized lint correction

Preserved original basea5c1a363 and #246af7bc6fd under
`backup/shared-ci-preflight-20261007/*` and the verified bundle
`.scratch/shared-ci-preflight-20261007.bundle`, SHA-256
`9e5c4a4d222c5b60e4d7e50af8a2ef9c66ed6bb46c8ba851f070fda1ea32cff9`.
Rebase onto #23749189160 completed with updateRefs/signing disabled.
The pure-foundation add/add conflict was a duplicate: the original composed
core tree equaled its old requester parent and the expected refreshed core
tree equaled the new requester parent. Skipped only that duplicate foundation;
Git also dropped the already-present storage patch. Template commands remain.

New prerequisite head28d7cdbbd3c89fd2f76e10850311f4c98f2da393 has exactly the
precomputed tree07ff2ed4009e2abd2570b432bbf8bf1dd1613e23. Rebased #246ecd5336a
has exactly expected tree790635c11e5343365495e8da99bd3fd7983a3c3d; its two original
patches are unchanged by range-diff. Only README and the two shared CI files
differ from the old heads. README and e2e match their owner blobs. CSV retains
its prerequisite-specific tests: its delta, not the entire file, matches #282
with stable patch-id9b983651968f82a97483988966201a5beb016a8a. Comparing the whole
CSV file to #282 correctly failed because that base lacks these existing tests;
no file was overwritten to force equality.

Commitc941040648306722ce6d2102ce5e43c79ad2cfac on #246 adds only three item-local
`cfg_attr(not(test), expect(dead_code, reason = ...))` annotations. Tests remain
compiled and unchanged, and all production queries, errors and authorization
logic are byte-identical. Expectations will become unfulfilled warnings when
real callers are connected, unlike a blanket allow. This preserves existing
unexposed work without adding a caller, activating policy or disabling Clippy.
Review used rust-best-practices lint/documentation guidance and ponytail.
Formatting56791 applied rustfmt;62664 then passed with zero changes.

Cherry-pickb2bb476a17798778d14d6ae75a1f1c783e9b8e0f carries exactly that commit
in the local complete composition. A safety review initially rejected the
write based on stale live-gate context; the actual50229 exit1 and an external
`ps -C nix` check proving no remaining Nix process resolved that concern before
the write. No worktree was edited during its gate. Clippy42781 now runs only on
the frozen #246c9410406 worktree; log `.scratch/preflight-c9410406-clippy.log`.
No full local gate is currently running. Next collect42781, verify the corrected
complete composition, publish the preflight refresh with pending checks explicit,
then continue dependent export/shared-CI refresh and final provenance review.
No merge or closure; the goal remains incomplete.

Clippy42781 subsequently exited1 on c9410406: expectations on the result struct
and error enum were unfulfilled once the dormant function had its own expectation.
No warning was disabled to hide this result. Follow-up9cc4330e removes those two
redundant annotations; the entire net change is seven lines on `preflight::read`,
with no query, test, data or authorization change. Complete compositionf90f60f7
contains the identical follow-up. Exact-branch Clippy58635 is running on9cc4330e;
log `.scratch/preflight-final-clippy.log`. Prior formatting69935 (complete) and
99150 (prerequisite) passed unchanged. Next collect58635 before publication and
resume the complete gate on the corrected composition, one full local gate only.

Clippy58635 exited0 on9cc4330e1dc8b425aa498206f595b681f800d93f, both horae-core
and horae with all native targets and warnings denied. Derivation
`/nix/store/gyia5krbv1as6csc0px2rhcxmdskrg7j-horae-clippy-0.1.0.drv`.
All paths other than the single preflight reader are identical to the rebased
extraction; its test-only build is unchanged. Both correction commits match
their composition counterparts by range-diff. Formatting3374 on complete
f90f60f7 and70602 on #246 passed with zero changes. No database/query cache
regeneration is needed for this annotation-only change.

Full gate38699 now runs on frozen complete composition
f90f60f754f359528c1ae08577d895f92bc5702b, using
`nix flake check -L --max-jobs 1 --cores 2`; log
`.scratch/permission-delivery-complete-f90f60f7.log`. The previous full gate and
both focused Clippy processes are terminal; there is only one local full gate.
Do not edit this worktree until that handle is terminal. No full pass is claimed.

Publication2238 succeeded atomically with exact old-head leases for preflight
base28d7cdbb and delivery9cc4330e. GitHub confirms #246 remains OPEN/DRAFT with
that exact head and unchanged review base name; the updated description records
historical failures, current Clippy success and pending full/remote gates.
Next: collect complete gate38699 without restarting on silence; refresh the
remaining prerequisite chains in dependency order and carry #263's explicit
denied-export rollback into #264 and descendants. A read-only inspection found
#264 still at6747c051 on base07684ac6, which contains old #26315bda5cc plus #249.
Neither branch was changed during this iteration. Preserve the rollback helper
when resolving its existing factoring against the refreshed export owner.

Post-publication inventory39281 confirms60 drafts,17 successful aggregates,
19 in progress and24 failures, with no local/remote head mismatch. Direct stale
review bases remain #272/#273/#275/#281; this check does not certify transitive
prerequisite freshness. #243354, #245355, #246356, #263350 and #283352 each have
successful evaluation and an in-progress build at their exact published heads.
Formatting20827 and whitespace passed for this ledger update. This iteration
is PROGRESS: preflight rebase published, exact-head lint corrected and verified,
and the full composition gate resumed. Completion remains unproven.

### Denied-export cleanup propagated through the report chain — 2026-10-07

The previous goal turn was PROGRESS (#246 published and verified by native
Clippy). This continuation re-read the saved objective, AGENTS, constitution
and applicable Rust/ponytail skills and reconfirmed #216 merged at02f7b58.
Complete gate38699 was confirmed live and its frozen f90f60f7 worktree was not
edited. Its client build completed, but the full gate remains pending. Read-only
process inspection confirmed active Rust compilation, not a stopped job; no
restart was attempted because of log silence.

The #264 review base still contained #263's previous15bda5cc head. Preserved
the base and delivery in `backup/export-cleanup-refresh-20261007/*` and verified
`.scratch/export-cleanup-refresh-20261007.bundle`, SHA-256
`40df3b5aa4101f41261aed0c43d8701254511d5c3e93640706fb1dfc4c1d2074`.
Preserved #265–#268 and the grouped prerequisite base in
`backup/export-cleanup-descendants-20261007/*` and verified
`.scratch/export-cleanup-descendants-20261007.bundle`, SHA-256
`441661f82e8f87e7de1640ec1325baeec19623ff99ddf91d25c7ab703153e66f`.
All worktrees and exact remote heads were checked before changes; rebases
explicitly disabled updateRefs and commit signing.

| Branch | Previous head | Published head |
| --- | --- | --- |
| integration/scoped-time-csv-prerequisites | 07684ac6 | e84f194367d9e5a080a12a084babe40dbe434aa5 |
| feat/scoped-time-csv (#264) | 6747c051 | 2820e56dbff0a417c7a1294cbdb5e41fcd2e3923 |
| feat/time-report-access (#265) | cec16d16 | 378462391f70c84351802a4f8106d64c9699703c |
| integration/grouped-time-export-prerequisites | 28bcf074 | 29f42408e74425de11f6f44a0fda27583bc411e9 |
| feat/grouped-time-exports (#266) | ae53be44 | 6293a081487699b012d0d4cda1620990542bfcd4 |
| feat/time-report-download-filters (#267) | 79798acf | 6c271f4191195e7a5853e54c380a4d724c5f7fb4 |
| feat/scoped-time-report-ui (#268) | 1be11e74 | 95e468218e9c5200bef21a41a042a964a323bb3e |

Replayed the two existing #249 patches over #26334341049 in the CSV review
base. The recovered limits-module conflict resolution was exactly original
bloba3a0bfa3. Final base treef38d9913117c4334fb4328b4e65695b458f59577 equals the
precomputed explicit-old-parent composition. The only range-diff difference in
the first prerequisite patch is existing module context; its cache patch is
identical. No descriptor or query was regenerated or replaced.

#264's factoring of `authorize_current` overlaps the same helper already moved
into #263 for the cleanup fix. Retained #263's explicit awaited rollback and
the original CSV `authorize_rows` factoring. Its final old/new tree delta is
exactly two files: the nine-line begin adjustment (one old line removed) and
the unchanged49-line regression. All other paths are byte-identical. No new
behavior, grant, API, dependency or source of product decisions was added.

#265's rebased patch is identical and treeb925ce92 equals its expected tree.
The grouped review base replays #262's existing patch; its recovered HTTP test
registration file is original blobbcce32d4 and its final tree4f7e4833 equals
the expected composition. #266 had an insertion collision between the new
regression and `mod grouped`; retained both unchanged, with the module declaration
before the test. Its range-diff changes only that insertion's context. #267's
patch and all three #268 patches are identical by range-diff; their final trees
6009ae6a and3a3eb8b1 equal the precomputed trees.

Every descendant differs from its preserved head only in the same two files
(57 additions, one deletion). Byte comparisons independently proved that both
the complete `begin` function and the regression function exactly match #263
in all seven updated branches. The helper delta's stable patch-id is
58919cf0f7faf0b711366967dc2cc0bb4e03aec2 at both ends of the chain. UI/CSS,
schemas, query caches and all other original tests remain unchanged. The
complete composition has additional report-consumer functionality in this file;
whole-file equality to it is neither claimed nor required.

Boundary review preserved denial semantics and database-error propagation when
rollback fails. The bounded export owns that transaction; streamed CSV retains
its existing close-on-drop connection and savepoint ownership unchanged. The
new inherited regression deterministically blocks pool cleanup and checks an
organization write lock with NOWAIT before releasing it. No sleeps, assertions,
timeouts or test registrations were removed to get a pass.

Formatting66178/#264,2856/CSV-base,79726/#265,75901/grouped-base,21105/#266,
22512/#267 and71783/#268 passed with zero changes; whitespace checks passed.
Focused Nix97638 runs on frozen #2642820e56d using disposable PostgreSQL,
the existing `reports::` tests and the HTTP export-filter module. Log
`.scratch/scoped-time-csv-cleanup-check.log`, recipe
`.scratch/scoped-time-csv-cleanup-check.nix`. It remains pending, not a pass.
No competing full gate was started;38699 remains the sole full local gate.

Publication65087 completed successfully: all seven branch updates were atomic
with exact previous-head leases. Updated #264–#268 descriptions preserve their
historical evidence, clearly label current checks pending and keep all five
PRs draft. Verified each exact remote head after publication. The new sections
do not claim that #282/#283 are inherited: that shared-prerequisite refresh,
retargeted CI and final cross-stack verification remain outstanding. No merge,
closure, original-worktree change or real-data operation occurred.

Next collect97638 and38699 without restarting live handles on silence. Record
the exact results, then refresh the remaining shared-CI prerequisite chains
in dependency order and reconcile the current-head inventory/provenance. This
iteration is PROGRESS, but the goal remains incomplete.

Post-publication inventory69702 found60 drafts:18 successful aggregates,
21 failures,20 in progress and one head without a build aggregate yet. No
local/remote head mismatch or accidental ready PR was found. Direct stale bases
remain #272/#273/#275/#281; transitive freshness still needs reconciliation.
The full composition advanced past the application package into Clippy, while
#26497638 advanced into runtime tests. Neither observation proves a full pass.

Focused97638 then exited0 on #2642820e56d:111 report tests passed, zero failed,
two existing manual export measurements ignored,994 other tests filtered out.
The inherited `denied_time_export_releases_authority_before_pool_cleanup`
regression passed. Its second command selected ZERO tests: `export_filters`
contains a helper called by the real `job_endpoints_enforce_session_role_and_organization`
test, not separately registered tests. This command provides NO HTTP evidence;
the zero exit status is not counted as covering that requirement.

Started a separate exact real-endpoint test on the unchanged #264 head using
`.scratch/scoped-time-csv-http-check.nix`; log
`.scratch/scoped-time-csv-http-check.log`. It invokes the HTTP matrix including
`time_reports::check` and `export_filters::check`, without rerunning the111
report tests. The shared transport correction #283 is still not inherited by
this branch; any failure must be retained and diagnosed, not retried into a pass.

The new HTTP-only handle is91192; #264 remains frozen until it ends. Focused
report derivation:
`/nix/store/sz64fc991rprh3s2rah07sbgga0pv1p4-horae-scoped-time-csv-cleanup-check-0.1.0.drv`.
PR status updates60632 distinguish the111 passing report tests from the zero-test
HTTP selector and the actual pending endpoint check. The next shared-CI review
base is `integration/materialized-export-prerequisites` at8c990cd6 (parents
99eab225 andc2d8de05), feeding #2474c6d0110 and #249c808b46c. It has only been
inspected: no rebase started there. Refresh it against its existing owners'
current heads before propagating CI changes through the exported report chain.

### Shared-CI export and reader prerequisites refreshed — 2026-10-07

The previous iteration was PROGRESS: export cleanup propagated and published
through #264–#268 with111 passing focused report tests. Re-read the saved goal,
AGENTS, constitution and ponytail; #216 reconfirmed merged at02f7b58. The complete
composition f90f60f7 remains frozen under live gate38699. Its native all-target
Clippy completed successfully and its browser checks are running; no full pass
is claimed. No extra full local gate was started.

HTTP-only91192 exited0 on #2642820e56d: the exact
`job_endpoints_enforce_session_role_and_organization` test passed, with1106
other tests filtered out. Its code calls the time-report and CSV/XLSX filter
matrices using real loopback HTTP and disposable PostgreSQL. Log
`.scratch/scoped-time-csv-http-check.log`, derivation
`/nix/store/3iyfv5c9c9syslna8clg7jxr593kzq4c-horae-scoped-time-csv-http-check-0.1.0.drv`.
The prior empty selector remains explicitly uncounted. PR updates28784 recorded
the actual pass;76682 clarifies focused success versus the still-pending full
gate. This does not prove #283 is unnecessary or certify any other head.

Preserved the materialized/CSV authority base and deliveries under
`backup/shared-ci-exports-20261007/*` and the verified
`.scratch/shared-ci-exports-20261007.bundle`, SHA-256
`c8406a383b83491da6b9d98a7b1e343a1c9c065ba737e10271666043a424c8d7`.
Read-only comparison ruled out replacing the existing composition by a single
unrelated independent PR. Reused `integration/invoice-authority-prerequisites`
99adead5: it matches old parent99eab225 except the two established CI files.
Replayed the original pure-domain/storage patches and #222's existing README
commit1fef1a24; no new implementation or requirement text was written.

| Branch | Previous head | Published head |
| --- | --- | --- |
| integration/materialized-export-prerequisites | 8c990cd6 | 267b0f0083dd9e7117ef4527c156dc7191af102f |
| fix/materialized-export-authority (#247) | 4c6d0110 | 9274066db2e1d182ac73f4078412b919726ae222 |
| fix/csv-export-authority (#249) | c808b46c | 5be60396c9c758346afb81bd05c9871d8e3fa950 |

The pre-README base tree16c8b64b matched the explicit-old-parent expectation;
final base tree40ecd449 matched the composition of current99adead5 and current
storage1fef1a24. Child trees648070b0 and28487084 matched their expected trees.
All four delivery commits are identical by range-diff. Every old/new tracked
path except README and the two CI files is byte-identical; those three match
their current owner blobs exactly. Formatting96476,99158 and23017 passed
unchanged. Publication97647 succeeded atomically with exact old-head leases;
descriptions92929 preserve historical checks and label fresh CI pending.

Preserved the editor base and four reader/editor deliveries under
`backup/shared-ci-readers-20261007/*` and verified
`.scratch/shared-ci-readers-20261007.bundle`, SHA-256
`0cd8b0a4ff9bf7adccf61b8049e0f575ab9a4144cdc14b14e5eba4b6d9d000cf`.
The old editor-base treea2b429f6 exactly equaled old #2457dc76ed3: its historical
merge added no unique content. Rebased that ref to current #245322f62b0 directly,
preserving the old merge in the backup rather than creating another composition.

| Branch | Previous head | Published head |
| --- | --- | --- |
| integration/permission-editor-build-prerequisites | a2b429f6 | 322f62b0b05af8b62209a86f6b02ab3201a3eaed |
| feat/permission-editor-api (#250) | 202532b8 | 39b3b1c3b4b7be5297c8de3cb6fb643763d20b25 |
| feat/scoped-people-directory (#253) | 80058039 | 1f6999549cd0f4b5238cf01a40223a1f9788a4cf |
| feat/project-people-picker (#254) | b9b47c09 | f93019fe972807ae91e44dcd0cec666e0c4d933d |
| feat/scoped-time-reader (#255) | e16978a9 | 40780585f4b9bf4a559e47a8a01fb3a23a2911ed |

All four delivery trees matched precomputed expectations (acf1654a,85332176,
b8e94925,de2001f1 respectively). All eight delivery patches are identical by
range-diff. Only README and the two shared CI files differ from preserved heads,
again matching their owner blobs; no production, query/cache, test or schema
change was introduced. Formatting71926,34304,62869 and7462 passed unchanged.
Publication44661 atomically updated all five refs with exact leases; each PR's
updated description and exact remote head/draft state were verified. #283 is
not yet inherited; current-head CI and final integration remain separate gates.
No merge, original PR closure, source-worktree mutation or real-data operation.

### Timesheet and report-reader shared-CI refresh

Preserved seven original heads under `backup/shared-ci-timesheets-20261007/*`
and verified `.scratch/shared-ci-timesheets-20261007.bundle`, SHA-256
`206d2397ad42db948a256b3403a009bcd41cf7fd609f58db42bb75634b2fcf0a`.
Local clean status and exact remote heads were checked before rebasing, with
updateRefs and signing disabled throughout.

The shared people prerequisite replays #253's unchanged directory/cache patches
over current #25540780585. Recovered conflicts in the HTTP registration and
profile-test files exactly matched original combined blobs9a2794dd anda1e156e1
before staging. Its final tree911c33fe equals the precomputed explicit-old-parent
composition; no novel source resolution was introduced.

| Branch | Previous head | Refreshed head |
| --- | --- | --- |
| integration/scoped-time-people-prerequisites | 2437326e | 91d21fc8391e431143f766e3bf1b2f61bf0335d6 |
| feat/timesheet-people-discovery (#256) | 5a95766b | f739f6d542425e97838c4f4f4d1b5f5493f1f2e0 |
| feat/timesheet-page-context (#257) | 657bf8ed | 50303263c884fa84eedb4abe698abb1ddda0e80a |
| feat/timesheet-person-commands (#258) | e1525af4 | 4d1fdb0a947b6c661a70d886caec62ecbe9627d7 |
| feat/timesheet-selected-person-ui (#259) | b94f2fb5 | 5469226f45a3ddbd24893566218b29ac4a3bd1cc |
| feat/scoped-time-report-reader (#261) | 46c0b1c9 | 06810c77d71d8ab851169f381a486b7787572555 |
| feat/scoped-time-report-groups (#262) | 91832917 | fb6e7b2d7e1abc1de9fd59686ece349447699f8d |

All six delivery trees matched their expected compositions:ff7dfdfd,1d42a6fe,
3a3ca73b,b6f69667,4547da44 ande19212a8. All nine non-merge delivery patches are
identical by range-diff. #258's old merge carried only #257's four-line query
barrier-release correction; that correction remains inherited from #25750303263
and is not duplicated or dropped by the flattened replay.

Every tracked file in all seven branches is byte-identical to its old head
except the two #282 recovery-test files and #222's README note; those match the
current owner blobs exactly. No UI, CSS, authorization, query/cache, schema or
assertion changed. Formatting17519/base,28642/#256,16334/#257,56806/#258,
66265/#259,27366/#261 and35635/#262 passed with zero changes. Publication46407
completed successfully, updating all seven refs atomically with exact leases.
All six PR descriptions were updated and their exact remote heads and draft
states were confirmed. Their current-head CI remains pending.

Complete gate38699 advanced beyond browser into SQLx preparation. Both exact
derivation outputs were independently confirmed valid via `nix path-info`:

- Clippy `/nix/store/ifqd0qb8fzm25x1bxdv7w8pkd839a711-horae-clippy-0.1.0`,
  from derivationnr7a2clln7q9rygp17x9hkp6lpiq148f.
- Browser `/nix/store/79f3zhvq2d4xg5rd2k3fnigbzna5fz66-horae-browser-checks`,
  from derivationgmlbhh5g7c5p07gd8yan9qpivyvnylr8.

These prove native lint and the checked-in browser suite on f90f60f7, not the
whole `nix flake check`, current remote heads, or an ARM gate. Keep the complete
worktree frozen while38699 remains live. Next collect the full gate,
reconcile a single current-head inventory, then refresh the remaining
report-export/editor/project composition bases in dependency order. Preserve
#263's explicit rollback and #246's dormant-reader annotation when carrying
existing fixes. This iteration is PROGRESS; the goal remains incomplete.

Post-publication inventory48370 confirms60 drafts with no local/remote mismatch:
14 successful aggregates,17 failures,25 builds in progress and four fresh heads
without a build aggregate yet. The lower success count reflects rewritten heads
awaiting new evidence; old green checks are not carried forward. Direct stale
bases remain #272/#273/#275/#281; transitive review compositions still need the
remaining refresh. The original #212 worktree remains atdb3935db with the same18
dirty paths; root still has only the original untracked `.playwright-mcp/`.
Ledger formatting67244 passed unchanged; no original files were removed.

### Report-export shared-CI refresh

Reconfirmed #216 merged at02f7b58a before changing branches. The preceding
status-only turn was a verified wait on live gate38699, not implementation
progress. This iteration advances the remaining report-export dependency chain.

Preserved all nine heads under `backup/shared-ci-report-exports-20261007/*`
and verified `.scratch/shared-ci-report-exports-20261007.bundle`, SHA-256
`d35640ad0b2a806513be6ed0edbf4ada8002672579929ff28c0402697b4f61e9`.
Each clean local head matched its remote before rewriting. Signing and automatic
rebase reference updates were disabled.

The XLSX review base now replays the existing legacy reader, financial snapshot,
materialized export and query-cache patches over current #26106810c77. Conflicts
only affected the combined HTTP test registry; the recovered file matched the
original blob138255d6 exactly. Its initial treeba516597 matched the explicit
old-parent composition. Formatting83234 then identified the original unsorted
module declarations. The existing #263 formatting commit15bda5cc was moved into
the shared base asd4a0d923, with identical patch and final registry blobd790cf33.
The duplicate downstream formatting commit was dropped because it is inherited.
All eight descendant trees remained identical across that move; no new source
resolution, missing tests or functionality was introduced.

| Branch | Previous head | Refreshed head |
| --- | --- | --- |
| integration/scoped-time-export-prerequisites | 83967167 | d4a0d923665fad210e4ac3d329609b015813fb05 |
| feat/scoped-time-xlsx (#263) | 34341049 | 4b43acf886f3f1bdc2d6ddbe12cdf0f0e8e0053a |
| integration/scoped-time-csv-prerequisites | e84f1943 | 38c0c28a105b9f42372b1044fa6d919780f57355 |
| feat/scoped-time-csv (#264) | 2820e56d | 1be265bd89a49aadd2447e28effbee781cac5230 |
| feat/time-report-access (#265) | 37846239 | fffa7194b702a1b7e7c68197f5a880adc41e466b |
| integration/grouped-time-export-prerequisites | 29f42408 | 3d7742bbffcc8127317423dae42e55a64aa65846 |
| feat/grouped-time-exports (#266) | 6293a081 | f6a604b260710a02e398578cc1d948d7c83bdaca |
| feat/time-report-download-filters (#267) | 6c271f41 | 16b773d53041bac2cd480381e6d5bf8c0f0dcd39 |
| feat/scoped-time-report-ui (#268) | 95e46821 | b797418348d8e8167a956b8b4d6900ce9d6a758a |

All expected descendant trees matched:df242181,b9329021,cbc25614,f7945c9f,
0f210d86,05508e7c,7f8313f3 andd968ab95 respectively. The final shared base tree
is80d2333b after moving only the existing formatting patch. All functional
delivery patches are identical by range-diff. Every tracked descendant file
is byte-identical to its preserved head except README and the two #282 recovery
test files; each exception matches its current owner blob. In particular,
#263's explicit denial rollback and regression are conserved throughout the
chain. Formatting63783 passed on all nine final worktrees with zero changes;
whitespace checks also passed.

The prior #264 reports and HTTP results remain valid evidence for their exact
older heads, not fresh full-gate evidence for these commits. #283 is still not
inherited by these branches. Review composition branches are not merge targets;
the six PRs remain drafts pending current-head CI and final integration.

Publication44560 completed successfully: nine refs updated atomically with
exact previous-head leases, and all six PR descriptions and exact remote
heads/draft states verified. No merge or original PR closure occurred.

### Shared editor composition refresh

Preserved the three original heads under
`backup/shared-ci-editor-composition-20261007/*`; verified bundle
`.scratch/shared-ci-editor-composition-20261007.bundle`, SHA-256
`5f8e4e4f1f3b23ad369576cc61e90b89408b78a54ae1675aa9168892f6c074da`.
Replayed the eight existing editor and identity-projection commits over current
#254f93019fe, with signing and automatic reference updates disabled.

Recovered editor conflicts matched original combined blobs0fc1f531 (HTTP test
registry),f40742ce (permission module) andba6d5f90 (profile tests). Identity
conflicts matched original blobsd9f9efee (HTTP registry) and6b556abb (users).
A transient MERGE_RR lock interrupted rebase bookkeeping; the lock disappeared
without deletion. The already-staged original identity change was committed
with its original message/author, then the remaining replay completed. Final
composition tree1bf76347 exactly equals the precomputed expected tree.

| Branch | Previous head | Refreshed head |
| --- | --- | --- |
| integration/permission-readers-editor-check | 8d57be90 | 794a8e26ee94d727b2fd4a4c96d28fe30eec6fd5 |
| feat/people-permission-editor-ui (#260) | f00d8f08 | b8bf4662fd2e2eb63ae7315a2e091d6949c21061 |
| feat/scoped-project-editor (#269) | 53183702 | 1442e01640f27be948cd60223f7055ac9f8b92e7 |

Both delivery trees match their expected compositions4aba6af5 and2bcf27ae;
all five delivery patches are identical by range-diff. Every tracked file in
all three branches is byte-identical to its preserved head except the two #282
test files and #222's README note, each matching the current owner blob. No
authorization, UI, CSS, test assertion, schema or query-cache change was added.

Formatting52998 passed on all three worktrees with zero changes; whitespace
checks passed. Publication71990 atomically updated all three refs with exact
leases, updated both PR descriptions, and verified remote heads/draft states.

### Project reader shared-CI refresh

Preserved the project reader and delivery composition bases and their two PR
heads under `backup/shared-ci-project-composition-20261008/*`. Verified
`.scratch/shared-ci-project-composition-20261008.bundle`, SHA-256
`160d59980d61e6e189562ac365c01b7e9c2f16bc6e01cc0b22dbcc32e76f9f6f`.
All four clean worktrees matched their remote heads before backup. Only the
reader base and #270 are updated in this step; #271 and its delivery base
remain unchanged at5beac2e1 and375e9bdf respectively.

Replayed the two original legacy/financial reader commits over794a8e26, retaining
the original combined HTTP test registry blob9f937242. New reader-base head
`c86806a8c3bfea2bce09f227f9af0972b1e4ee3e`, treea68d1306, matches the expected
explicit-old-parent composition. #270 now has head
`84b98033b2771a39b6004217d32a5ae98a1f7234`, expected treedf63c7ed.
All six functional/project-test/query-cache patches are identical by range-diff.

The old CI commits737aa13a andb480f9d2 already exist in the refreshed dependency
through #282. Before skipping the conflicting diagnostics replay, confirmed
that the current parent's entire e2e file equals both #282 and old #270. The
ARM timeout replay then dropped automatically as already upstream. Nothing
was removed from the final file. All other files are byte-identical to their
preserved heads except README and the shared CSV recovery fixture; the base
also gains the shared e2e corrections. Each inherited exception matches its
owner blob. No production, UI, query, migration or assertion change was added.

Formatting33003 passed on both worktrees with zero changes. Publication44940
updated both refs atomically with exact leases, updated #270's description and
verified its exact remote head/draft state.

The complete gate38699 remains live on frozenf90f60f7 and has advanced into the
server test suite. SQLx preparation is now independently confirmed successful:
`nix path-info` accepted output
`/nix/store/hggbpakr3dcwbfd06ab4slgga6s82dkp-horae-sqlx-prepare-0.1.0`
from derivationsyd8gb6brc2bzv66gfd7aajxrk6s4zfc. This is exact-head SQLx evidence,
not a claim that the remaining tests/deployment gates passed or that remote
ARM builds passed.

Next: collect gate38699 without restarting it; refresh the already-backed-up
project delivery composition375e9bdf and #2715beac2e1, preserving #263's denial
rollback from current #26716b773d5 and the verified #270 reader changes. Then
refresh the remaining task/Harvest descendants and reconcile the final
provenance and current-head verification inventory. #283's transport fix and
#246's dormant-reader correction remain explicit integration requirements.
This iteration is PROGRESS: nine PRs and five review bases updated and published;
the goal remains incomplete. No merge, original closure, real-data operation,
source-worktree cleanup or new feature was performed.

Post-publication inventory18761 reports60 drafts, no local/remote head mismatch,
14 successful aggregates,17 failures and29 in progress. These are GitHub check
summaries, not proof of merge readiness; underlying build commit identity must
also be checked. Direct stale bases are #272/#273/#275/#281; remaining review
compositions also require transitive refresh. Original #212 stays atdb3935db
with the same18 dirty paths, root retains only `.playwright-mcp/`, and the full
gate worktree is clean atf90f60f7.

Two failure logs were inspected once to refine the remaining CI work:

- Nixbot327 belongs to current #2221fef1a24. Its server suite had829 passes,
  one failure and11 ignored: the real HTTP test's CSV CLI invocation returned
  `indeterminate_submission` / exit6 instead of the expected denial / exit1
  at `authorization_tests/cli.rs`. This matches the previously reproduced
  transport-framing failure signature; #283 must be composed and checked rather
  than relaunching the unchanged failing head. Evidence:
  `.scratch/nixbot-327-current.html` and `.scratch/nixbot-327-tests.log`.
- The aggregate linked from #223 points to Nixbot323, whose page identifies
  old head1dc4b6bc, not currentbc74af8d. Its ARM OIDC VM timed out at900 seconds
  waiting for the test shell, while PostgreSQL was still initializing around
  guest second885. No OIDC assertion had run. This does not prove a current-head
  application regression or success; reconcile the build/check attribution and
  investigate the boot-time limit before proposing another CI change. Evidence:
  `.scratch/nixbot-323-current.html` and `.scratch/nixbot-323-oidc.log`.

These findings add explicit transport integration and ARM/build-attribution
follow-ups to the next actions above; no failed test was disabled or retried.

### Project export composition refresh

The previous goal iteration was PROGRESS. Reconfirmed #216 merged before
rewriting code and reused the verified project-composition backup recorded
above; both old heads still matched their remote refs and clean worktrees.

Rebased the existing project-delivery composition over current #26716b773d5.
Duplicate legacy readerae152748 and financial reader6bf2fbcf were omitted only
after verifying the former differs from inheritedb761912a solely by four SQLx
descriptors already present byte-for-byte, and the latter has the identical
patch as inherited99eab225. The final existing test registration order was
restored explicitly. Resulting tree7f94aa22 exactly matches the precomputed
explicit-old-parent composition, including the existing #263 denial rollback.
Then replayed #270's existing cache cleanup0b4fd421 as71a84073; range-diff is
identical, removing the same six obsolete generated descriptors. They remain
recoverable in the original commits and verified bundle.

The refreshed review base is
`71a840734ffc5a3e223c52ad54cb22215cacb6df`, expected tree9ec0559f. #271 is
`d4a4af79e3347bbb859baae2263d4294419382ca`, expected treed0ef3839. Its two
original delivery patches are identical by range-diff. Every other tracked
path matches the preserved head except the two known rollback files, the six
known SQLx deletions, the two #282 recovery-test files and #222's README note.
The rollback files exactly match current #267; CI/README exceptions exactly
match their owners. No new authorization or product behavior was introduced.
Formatting62147 passed on both branches unchanged. Publication65883 atomically
updated both refs with exact leases and verified #271's new head/draft state.

### Complete x86_64 Linux gate passed

Session38699 exited0 on the unchanged, clean complete composition
`f90f60f754f359528c1ae08577d895f92bc5702b`. Its command was
`nix flake check -L --max-jobs 1 --cores 2`; the retained log
`.scratch/permission-delivery-complete-f90f60f7.log` ends `all checks passed!`.
This covers the composed build, checked-in browser suite, Clippy, SQLx cache,
server/integration tests and NixOS deployment/OIDC checks on x86_64 Linux.

The main server suite reports1478 passed,0 failed,11 ignored, followed by all
15 integration/UI test binaries passing. The11 ignored tests are the existing
manual scale/memory/export measurements, not newly disabled failures. Four
focused one-test runs also passed. The tests derivation output was independently
confirmed valid by `nix path-info`:
`/nix/store/h7jfb54lcgdq84dbc047j9928sp1y1n1-horae-tests-0.1.0`, from
derivationp2jaidgz0npdlq2dcadnjjcjf8qpfjj7. Earlier exact-head Clippy/browser/SQLx
evidence remains valid. The process is terminal; do not poll or restart it.

The flake explicitly omitted incompatibleaarch64-darwin andaarch64-linux.
This does not prove ARM CI, current individual PR heads, or final provenance
reconciliation. The passed composition contains #283 and #246's corrections;
those must remain accounted for when reconciling refreshed extraction heads.
The goal remains incomplete despite this full local pass.

### Harvest and task reader prerequisite refresh

Preserved four clean, remote-matched heads under
`backup/shared-ci-harvest-task-readers-20261008/*`; verified bundle
`.scratch/shared-ci-harvest-task-readers-20261008.bundle`, SHA-256
`6b56071a2ea4ccf1eed10f7353bcdb2f850acd322b9c2eabbc01dd5e0c721e3f`.
Rebased each PR onto its actual current parent, incorporating the previously
separate SQLx cleanups rather than continuing to use the pre-cleanup parent.

| PR | Previous head | Refreshed head | Expected tree |
| --- | --- | --- | --- |
| #272 | 8b8b0c0b | f10a735bb81fac091f982dce2ef11a18b44176fb | 1d7e8df3 |
| #273 | 65c4aa23 | d2c0507fd5c3e2b379d94cbedb8c46f6ee4f77ce | 4c1344d7 |
| #275 | f8c1477b | 431417c0f35a197cee1db0a7a36c42d926e8ee9f | 5882d0f4 |
| #281 | cd8d1db7 | 31cd51af84696d7647bcdaee9a622aff3f02fc9f | c79eed1e |

All nine delivery patches are identical by range-diff, and every tree equals
its precomputed explicit-old-parent composition. Every tracked source, test
and UI file is byte-identical to the old head except the two #282 recovery
fixtures and #222's README note, which match their owner blobs. The SQLx
differences are exactly6/9/10/9 descriptor removals respectively, independently
compared against the union of existing owner commits0b4fd421,8b8b0c0b and
65c4aa23 as applicable. No unrelated descriptor was removed and no SQL changed.
Formatting11791 passed on all four final worktrees with zero changes.

Publication22902 atomically updated all four refs with exact leases; every PR
description and exact remote head/draft state were verified. These remain drafts
pending fresh current-head CI; #283 is not inherited by them yet.

### Task delivery dependency refresh

Preserved seven clean, remote-matched heads under
`backup/shared-ci-task-delivery-20261008/*`; verified bundle
`.scratch/shared-ci-task-delivery-20261008.bundle`, SHA-256
`6027eecb64036fa8b4b1f3babba8fc7eb6d04343c5c825fdedaee7e0ee12bb5a`.
The lifecycle base replays its12 original editor, Timesheet and import commits
over current #275431417c0. Recovered browser-runner/navigation blobs match the
originalc1d60a8d/e7c1ad04, and final HTTP/profile registries match original
463e7c04/6845a836. Initial treea2792596 equals the precomputed composition.
Then inherited #269's existing one-descriptor cleanup53183702 asce06d51c,
yielding expected tree66635fd6. No new semantic resolution was introduced.

| Branch | Previous head | Refreshed head | Expected tree |
| --- | --- | --- | --- |
| integration/task-lifecycle-prerequisites | 1955c38f | ce06d51c1f0ec81d7904c9f6c81184671ea719cf | 66635fd6 |
| feat/scoped-task-lifecycle (#276) | 7e883eb5 | 14e2c72862a2e1079d8a4b64a0cb1b7f0921b7a6 | 3b82c512 |
| feat/scoped-task-links (#277) | d1ab5224 | 598b2dd9724dae3991d96f5a3da7ea20b53d8cb6 | cbf95029 |
| feat/atomic-task-creation (#278) | 812a870f | 11c87a95b56bc78ddd7c08fe5cc89116d2ba05de | d9db2431 |
| integration/task-catalog-prerequisites | e9f793df | b6fbf9e7541c0708e4079a0c01a410c073261320 | 0c5e207c |
| feat/task-catalog (#279) | 9fcc0a09 | 1cb4edc56baea2da9b7231797c544ef38f8ec0f1 | 96832117 |
| feat/project-task-activity-ui (#280) | 24c5d0e1 | 810449aaa74f440743bd55e75eb50b443ddbeb4b | 7c22636b |

The catalog base replays the existing People editor UI over the current atomic
creation branch. Its two recovered files exactly match original blobs2c554ac9
(admin page) andf10c5a0a (browser runner). Every final tree matches its expected
composition. All six delivery patches across #276/#277/#278/#280 are identical
by range-diff. #279's sole delivery patch differs only because its removal of
the obsolete TaskRateEdit lint annotation is already inherited from #278; the
final file is byte-identical to old #279. The older review merge is preserved
in the bundle, not retained as a redundant merge in the refreshed branch.

Every tracked file across all seven branches matches its preserved head except
README, the two shared recovery-test files and11 known SQLx removals. The SQLx
sets exactly equal the existing cleanups0b4fd421/8b8b0c0b/65c4aa23/53183702.
README/e2e match their current owners; the CSV fixture matches current #231,
which includes its original transaction-boundary tests as well as the #282
cleanup correction. Comparing that whole file to #282 alone would wrongly omit
the already-existing #231 tests; the earlier owner-comparison failure made no
source change. No test, assertion or feature was removed.

Formatting1192 passed on all seven final worktrees with zero changes; whitespace
checks also passed. #283 was checked once after the complete local gate ended:
GitHub still reports Nixbot352 IN_PROGRESS on exact head8d83b853, with no failed
check in its current rollup. No retry was requested; this is not a completed
remote gate or ARM evidence.

Publication46213 completed successfully: seven refs updated atomically with
exact leases, all five PR descriptions updated, and remote heads/draft states
verified. This iteration published10 PRs and three review bases without merging.

Post-publication inventory77241 reports60 drafts, no local/remote head mismatch
and no stale direct bases. Aggregates are9 successful,15 failed,32 in progress
and4 missing after recent rewrites. These are status summaries, not proof that
the underlying Nixbot link belongs to the current head; the attribution caveat
recorded for #223 remains. Remote master is still8b3cc257. Original #212 remains
atdb3935db with the same18 dirty paths; root retains only `.playwright-mcp/`.

### Refreshed-head content reconciliation

Generated `.scratch/permission-delivery-refreshed-coverage.json` against the
exact passed compositionf90f60f7 and all60 current extraction/correction heads.
This is a content-coverage audit, not an ancestry claim: flattened/rebased
review histories do not make the refreshed commits literal ancestors of the
already-tested composition.

The audit finds1159 exact single-variant paths and108 shared paths. Of the
shared paths,81 match a current input blob; the27 composed blobs exactly match
the earlier audited compositionefae7522, so no fresh source resolution was
introduced. The seven non-exact single-variant paths are precisely the six
previously audited query replacements and the existing scoped-report fixture
adaptationaf7c35dc. Each superseding source blob is unchanged from the earlier
audit, and the adapted fixture matches its recorded blob91f508aa. There are
zero unaccounted paths in this comparison. The original audit artifact was
retained rather than overwritten.

An explicit registration-union check over all60 heads also confirms the passed
composition retains all34 browser suites and25 HTTP matrices, without missing
or duplicate registrations. This supplements the blob comparison; it does not
turn unexecuted manual measurements or remote ARM checks into passed gates.
The reverse path check also found zero target-only files: no composition file
falls outside the baseline and all current input trees.

Next: finish current-head CI/build attribution and ARM boot analysis, integrate
#283's transport correction into the necessary prerequisite roots without
changing features, and recheck exact affected heads. Reuse the successful
f90f60f7 integration evidence where its exact tree still applies; do not restart
its completed process. Reconcile final PR readiness and the incomplete-work
inventory before claiming completion. The goal remains active/incomplete;
this iteration is PROGRESS, not a wait or a completed delivery claim.

### ARM boot attribution and store-image candidate — 2026-10-08

Revalidated #216 as merged at02f7b58a before creating a new isolated worktree. Original worktrees
and completed local gate38699 were not changed or restarted.

Resolved the earlier #223 attribution caveat: GitHub check113008754898 attaches
build323's failed ARM OIDC result to currentbc74af8d. Evaluating that exact
head yields `/nix/store/d1n5iwcsxxy9zka3ah3qbgzcsbqckypi-vm-test-run-horae-e2e-oidc.drv`,
identical to the failed log. The build page's older commit label is not grounds
to dismiss this failure.

Read the actual terminal OIDC log for build352 on #2838d83b853, rather than
restarting its still-running aggregate. It failed at the900-second control-shell
readiness wait. Horae was already listening at guest546.76s; a virtio keyboard
appeared only at879.63s. No OIDC assertions were reached. The failed derivation
is `/nix/store/i2kgrw4v0kpg431dap9c9nc2157mcw75-vm-test-run-horae-e2e-oidc.drv`.
Raw evidence: `.scratch/nixbot-352-arm-oidc-complete.log`.

Compared successful build317's raw OIDC log, retained as
`.scratch/nixbot-317-arm-oidc-complete.log`: Horae listened at429.66s, keyboard
appeared at521.50s, backdoor started at570.97s, and the full test finished in
618.52s. Both runs explicitly report unavailable KVM and fallback to TCG.
The pinned test instrumentation requires hvc0 and ttyAMA0 before starting its
control shell; the late keyboard event is not itself proof of a keyboard bug.

Candidate branch `test/vm-store-image`, worktree `.worktrees/vm-store-image`,
starts at #2838d83b853 and enables `virtualisation.useNixStoreImage` in the two
NixOS checks. Four added lines include comments; there are no application,
assertion, timeout or platform changes. The pinned qemu-vm module documents
this option as replacing9p store reads with a local disk image, at the cost
of image construction time and disk space. This is a performance candidate,
not yet a proven remedy for the remote failure.

Scope evaluation47541 passed for x86 and ARM: both application derivation paths
and both complete test scripts are unchanged. Evidence is
`.scratch/vm-store-image-scope-check.json`. ARM evaluation39555 confirms the
image is enabled, the host9p store is disabled, and requiredFeatures still
include kvm and nixos-test. Formatting passed with zero changes. Local complete
NixOS checks74569 are running; log `.scratch/vm-store-image-x86-check.log`.

A local ARM-driver dry-run81989 succeeded after correcting the diagnostic
expression to use `extend.modules`. It would require277 derivations and420
substitutions; this host has neither an ARM builder nor ARM binfmt registration.
No speculative local ARM build was started and no emulation settings were
changed. Fresh remote ARM execution remains necessary.

Initial checks74569 completed with exit0, but log inspection found an unacceptable
VM regression: register-nix-paths failed because the image-backed store lacked
the writable overlay. The original9p configuration provided that overlay by
default; useNixStoreImage changes the default. This candidate was not published.
Preserved the initial log and scope evaluation instead of treating green
application assertions as proof of a healthy VM.

The corrected candidate explicitly preserves `virtualisation.writableStore`
and adds `wait_for_unit("register-nix-paths.service")` in both scripts. It adds
ten lines across two files, without deleting or weakening any existing check.
Scope evaluation54362 confirms unchanged application derivations and identical
old test scripts after removing only the new registration guard, on both Linux
architectures. The writable overlay is enabled on both. Evidence:
`.scratch/vm-store-image-overlay-scope-check.json`.

Corrected checks8383 passed completely: OIDC30.80s and deployment/recovery84.17s,
both with successful store registration and its explicit guard. Formatting
passed with zero changes. Committed93b3fbaa3a988b5dc6b05aa930fa202d4198f335
without signing; push30413 succeeded and draft
[#284](https://github.com/numtide/horae/pull/284) was created over #283.
The ten-line diff uses native VM options; review found no application,
authorization, timeout, assertion-removal or platform change. Image generation
adds temporary disk/time overhead, so remote ARM benefit remains unproven.

Full local `nix flake check`34073 passed against published93b3fbaa; log
`.scratch/vm-store-image-93b3fbaa-full-check.log` ends in all checks passed.
It reused unchanged derivations, built the new formatting gate, and explicitly
omitted incompatible ARM/Darwin systems. GitHub verification36118 confirms
the exact draft head/base and Nixbot402 IN_PROGRESS. This is not a completed
remote gate or ARM performance proof. Keep completed
compositionf90f60f7 as content/integration evidence, not proof of the new VM
configuration. No merge or extraction-head rewrite was performed.

The refreshed remote snapshot40438 is retained at
`.scratch/permission-ci-20261008-boot-review.json`. It lists60 scoped open PRs.
Inspected four additional x86 test logs instead of assuming every red check
was the ARM boot failure:

| Build / PR | Observed failure | Next action |
| --- | --- | --- |
| 340 / #234 | CLI CSV dry-run returns indeterminate_submission exit6 instead of1 | Same signature as #283; propagate the verified framing correction |
| 330 / #231 | reclaimed-lease CSV test unwraps Elapsed at csv_streaming.rs:563 | Inspect its synchronization separately; not fixed by a VM storage change |
| 337 / #236 | CSV resume test fails its durable-batch-before-EOF assertion | Inspect the batch observation/worker outcome before attributing it to load |
| 338 / #228 | cancelled-preview CSV test fails its durable-batch-before-EOF assertion | Same fixture boundary class, still unresolved |

Evidence files are `.scratch/nixbot-{330,337,338,340}-x86-tests-tail.log`.
These are identified build-log failures, not a claim that all four derivations
have been reconciled against current heads. The three CSV fixture failures
need independent diagnosis before another broad shared-prerequisite refresh.

Next: collect #284's fresh ARM result; diagnose the three CSV fixture
failures independently, then choose the verified shared prerequisite base for
#283 propagation. The goal remains active and incomplete. This iteration made
progress through fresh failure attribution, the reviewed VM candidate and its
two passing local deployment checks; no broad CI-readiness claim is made.

### CSV deadline attribution and runner allocation — 2026-10-08

The preceding iteration made progress through published #284 and its complete
native gate. Revalidated #216 as merged before creating isolated diagnostic
and runner worktrees. Nixbot402 on exact #284 remained IN_PROGRESS; no retry
was requested. Its ARM OIDC raw endpoint had no output yet, not a failure or
completed boot result.

The three CSV failures are current-head evidence. Evaluating Git flakes from
their worktrees gives exact matches to the failed logs:

| PR / head | Build | x86 tests derivation |
| --- | --- | --- |
| #231110b365a | 330 | l658jk2f4lj6v1agn81s6a490xn1fv4x-horae-tests-0.1.0.drv |
| #2365f0c619c | 337 | d8bb6zr41qdjzpjqc0snwisi8il5jm96-horae-tests-0.1.0.drv |
| #228951baf9d | 338 | v0f6dz6flkg3qn96ccbz6k8jqb39p0m6-horae-tests-0.1.0.drv |

Build-page commit links also match those heads. An initial absolute path-flake
evaluation produced different derivations; it is not the Git-flake attribution
proof. Build338 includes both cancelled-preview and resume-before-EOF failures,
not only the first failure listed in the previous summary.

The shared interruption helper waits10s for a traced commit, but previously
aborted without reporting the worker outcome. Diagnostic worktree
`.worktrees/csv-batch-readiness` adds only failure diagnostics and runs the20
streaming tests through35205; log `.scratch/csv-batch-readiness-diagnostic.log`.
This diagnostic source change is not part of any published PR.

Diagnostic35205 finished exit0:16 passed,4 existing manual100k measurements
ignored,812 filtered out. The selected streaming tests did not reproduce the
full-suite failure, so no timeout root cause is claimed from this run. Saved
the exact diagnostic patch at `.scratch/csv-batch-readiness-diagnostic.patch`
and restored the diagnostic worktree to its original clean93b3fbaa after the
process completed. Its log remains available; no production/test assertion
change is being published from that experiment.

A separate sandbox probe96903, using the same `--cores 2` request, reports
`nix_cores=2 rust_test_default=32`; log
`.scratch/test-runner-parallelism-probe-with-linker.log`. Thus the checked-in
runner ignores the per-build test concurrency budget. This demonstrates the
allocation mismatch, not yet that it explains every CSV deadline failure.
The pinned stdenv setup normalizes zero/unspecified NIX_BUILD_CORES to a positive
value; no custom normalization or deprecated RUST_TEST_THREADS setting is needed.

Isolated branch `test/nix-test-parallelism` adds explicit test-thread flags to
nextest and libtest in one Nix file. Formatting and whitespace checks passed.
Core tests passed121/121, and full app suite57770 is executing with
`--test-threads 2`; log `.scratch/nix-test-parallelism-tests.log`. All test
contents, limits and internal concurrency remain unchanged. The unsigned
commit15d9ab5b6ed18f0b177d425e0232da9d33f23346 was published via89033 and draft
#285 opened over #284. Prerequisite order remains #282→#283→#284→#285.
Verification84488 confirms its exact head, base and draft state; Nixbot404
started evaluation on that head. This is not a completed remote check.

Next: collect57770, then reconcile #284/#285 remote CI against exact
heads. Keep CSV failures open until adequately verified; do not propagate a
speculative remedy across all extraction heads. Original work and completed
gate evidence remain preserved; no merge or product change was made.

### Consumer ownership reconciliation and bounded runner result — 2026-10-08

The preceding status turn was a verified wait: session57770 was confirmed live,
and its log had advanced from compilation into server tests. This iteration
collected its terminal exit0. The published #285 head15d9ab5b is unchanged and
its worktree is clean. The log records121 core tests,821 server tests with11
existing ignored measurements, and all auxiliary binaries passing; four
isolated child-test results are not counted again as additional server tests.
Full native flake check79988 passed against that exact published head, logged
at `.scratch/nix-test-parallelism-15d9ab5b-full-check.log`. It reused unchanged
derivations and the completed tests output, built the new formatting check,
and explicitly omitted incompatible ARM/Darwin systems. Remote Nixbot402/404
remain IN_PROGRESS on exact #284/#285 heads; their evaluations passed. Neither
ARM readiness nor resolution of the three earlier CSV failures is proven yet.

Corrected stale original-commit rows that still described Reports and Tasks
consumers as awaiting extraction. Read-only Git blob comparison33b690 confirms
all six production paths below are byte-identical to originaldb3935db:

| Delivery / current head | Original production paths |
| --- | --- |
| #268b7974183 | `pages/reports/scoped.rs`, `pages/reports/scoped/grouped.rs`, `pages/reports/scoped/expanded.rs` |
| #2791cb4edc5 | `pages/tasks.rs`, `pages/tasks/editor.rs` |
| #280810449aa | `pages/new_project/tasks.rs` |

Paths are relative to `crates/horae/src/`. Reports browser differences add only
the two previously documented SVG direction assertions. Its component fixture
adapts legacy identity/tag stubs to the extraction base, without removing test
assertions. This reconciles ownership, not new runtime verification. Existing
backend and contract ownership in #275–#280 is unchanged. No implementation,
original branch, incomplete feature or acceptance gate was altered.

Spec Kit skills are unavailable in the current session/environment; this narrow
Git/documentary reconciliation is not reported as a new Spec Kit execution.
Earlier specification analysis and its limitations remain in this ledger.
Next: collect the exact-head remote ARM results, then propagate only
verified CI prerequisites and finish current-head readiness reconciliation.

### Shared browser ownership and complete CI composition — 2026-10-08

The preceding iteration made progress: full native #285 verification completed
and ownership corrections were published in #218 at26859d4a. Current Nixbot
402/404 remain live on the same exact heads. Their pending VM jobs are not
treated as failed or restarted.

Read-only comparison closes three remaining ambiguous browser ownership rows:

- Original4294aa3 has exactly three paths. The browser.nix asset-root change
  belongs to #259/#260; permission-editor-recovery.cjs belongs to #260; the
  manager fixture and single-admin assertion in new-project-permissions.cjs
  belong to #269. The latter two whole files have zero diff against db3935db
  on current heads b8bf4662 and1442e016 respectively.
- Original2f5357f's96 added recovery/history-browser lines remain inside the
  exact #260 fixture (blob b84c961b9c0d756d8d983cecb82a6b6d667c4443), also exact
  in compositionf90f60f7. The other four browser paths' Timesheet readiness and
  picker changes belong to #259. Their differences from the original are the
  already-recorded master Clients integration and separate Project overview
  endpoint, not deletion of the Timesheet assertions. Its four spec paths are
  owned by #248.
- Compositionf90f60f7 retains the complete original navigation dispatcher plus
  master's Client guard. Its13 navigation tests cover pending and dirty states
  for Permissions, Clients, Invoices, Projects and Timesheet, both permission
  and Timesheet scroll-only replacements, and legacy explicit release. The
  unchanged full-gate log records13/13 passing and successful real browser
  history/recovery checks. This closes the local union check, not remote
  acceptance of later rebased heads. No repeat browser run was necessary.

After reconfirming #216 merged, created isolated
`.worktrees/permission-delivery-ci-check` on branch
`integration/permission-delivery-ci-check` from preservedf90f60f7. Cherry-picked
only #284/#285 as unsigned2474cb45 and604c0481: three Nix files,13 additions and
two replaced lines. All application code, assertions, database schema and
original worktrees are unchanged. The previous verified composition remains
clean and intact. This branch is a local verification artifact, not another
delivery PR or a replacement for the independent extraction branches.

Full native `nix flake check -L --max-jobs 1 --cores 2` runs as14504 on604c0481;
log `.scratch/permission-delivery-ci-604c0481-full-check.log`. This exercises
the bounded runner against the full permission composition as well as the VM
store options; no completed result is claimed yet. Next collect this handle
and remote402/404, then select verified prerequisites for the extraction
refresh. No merge, closure, policy activation or product change occurred.

### Retained full-feature acceptance classification — 2026-10-08

The preceding iteration made progress through the browser ownership audit,
published ledgerab2f2750 and isolated CI composition604c0481. Session14504
remains live, now compiling the full app after191/191 core tests passed with
the explicit two-thread budget. This is not a complete composition result.
Remote402/404 remain IN_PROGRESS on unchanged published heads; no retry or
branch rewrite was requested.

Current remote inventory aafeac confirms58 original-work extraction PRs plus
four CI prerequisite PRs, all open drafts. Replaced the outdated31-candidate
description and unassigned-behavior placeholder with actual ownership and
retained follow-up semantics. Original #212 still has the same18 dirty paths;
the root's unrelated untracked directory remains untouched.

Read the source's30 unchecked task definitions and their explanatory context.
Added one classification table in this ledger distinguishing extracted
sub-increments from unfinished implementation, umbrella acceptance and
governance/cutover work. In particular, Task catalog and ordinary Reports UI
are already extracted; their unchecked broad tasks are not evidence those
screens are absent. Conversely, extracted compatible client reads do not
deliver T238's ordinary Clients workflow. Profile/audit sub-increments do not
close all profile, disclosure, design or activation requirements.

Read-only check5c9006 compared every source unchecked ID against the new table:
30 source IDs,30 classified IDs, no duplicates and exact set equality. This
checks bookkeeping coverage, not requirements satisfaction. No task checkbox,
spec contract, original document, source code or acceptance threshold changed.
This is documentary reconciliation, not a new Spec Kit execution; its skills
remain unavailable in this session. The goal may finish with explicit retained
unfinished work, but required extraction CI/review still prevents completion.

Next: collect14504 and402/404; propagate verified CI prerequisites without
restarting live jobs, then reconcile readiness against the refreshed heads.

### CI propagation preview without branch rewrites — 2026-10-08

The preceding iteration made progress by publishing the retained-acceptance
classification at ec38c7b5. Reconfirmed remote402/404 are IN_PROGRESS on the
same #284/#285 heads. Local composition14504 is still compiling; live rustc
processes consume CPU, so no timeout or stopped-build inference is warranted.

Inspected the earlier refresh scripts without executing them: their original
heads, leases, target and recovery namespace are stale. They must not be reused
blindly. Global rebase.updateRefs remains true; any subsequent rebase must
explicitly disable it to avoid moving preserved/neighboring refs.

Previewed the candidate CI base15d9ab5b with git merge-tree. The57 code
extractions compose without conflicts and change only the five expected paths:
CLI CSV transport/tests, two VM checks and the Rust test runner. A second
preflight included all18 integration branches currently used as review bases.
Every one of the75 branches was clean and matched its actual remote ref.
All75 resulting patches are byte-identical to #282→#285's77 additions/two
replacements; all57 review diffs remain byte-identical when their corresponding
bases are updated. No feature hunk, SQLx descriptor, migration or browser
assertion enters the CI patch. Documentation-only #248/#218 are outside this
code-refresh preview, and originals #212/#217/#208 are excluded.

Evidence: `.scratch/permission-ci-propagation-preview-20261008.json` records
each branch, worktree, old/remote head, expected tree, base and review-diff
comparison. It is a simulation, not an executed rebase, test result or
authorization to publish without revalidation. Git tree objects were created;
no refs, worktrees, PR bases or remote branches were changed. Before actual
propagation, revalidate these heads/leases, preserve fresh recovery refs/bundle,
retain dependency order and compare every resulting tree and review diff.

Next: collect14504 and402/404; use this verified structural preflight only after
the candidate's required CI evidence is adequate. Avoid starting another
repository-wide refresh while that base remains under verification.

### First bounded-runner ARM success — 2026-10-08

The preceding iteration made progress through the75-branch propagation preview
and published ledgera92c68fd. Confirmed session14504 live and remote402/404
active, then used timed waits rather than restarting compilation or polling
all PRs. ARM402 advanced from queued VM checks to building the new guest
image; it had not yet reached the boot assertions. Its elapsed build time is
not a new900-second guest-readiness failure.

Nixbot404 now reports checks.aarch64-linux.tests as succeeded. The exact-head
log records both runners using8 threads,121/121 core tests and821 server tests
passing with11 existing ignored measurements, followed by all nine auxiliary
binaries. Four nested child-test summaries are not double-counted. The server
suite took92.56s; buildPhase6m8s. ARM formatting also succeeded. Evidence is
captured in `.scratch/nixbot-404-arm-tests-summary.json`, with public log/status
URLs and the observed head. Updated #285's verification without claiming the
remaining VM/full-CI result or universal resolution of earlier CSV failures.

Local full composition14504 finished compilation in8m22s and began1489 server
tests (including the existing11 ignored measurements). The durable CSV
cancelled-preview, reclaimed-lease and resume-before-EOF cases have passed in
this wider composition. The full suite and subsequent VM checks are still
running; source-level framing and rollback corrections remain included.
No assertion, timeout, branch, dependency or original file was changed.

Next: collect14504 and the402/404 VM results. Keep the propagation preview
intact and do not publish rewritten extraction heads until their shared base
has adequate verification. The goal remains active, not complete or blocked.

### Recovery checkpoint before CI propagation — 2026-10-08

The preceding iteration made progress through exact-head ARM test acceptance
and published ledgerd710a8a9. Confirmed composition14504 and remote402/404 live;
the combined server suite continues through permission transaction tests.
ARM VMs are building guest dependencies, not reporting completed boot checks.

Before any refresh, revalidated every one of the75 preview branches against
its current local head, actual remote ref and clean worktree. Created recovery
refs atomically under `backup/ci-propagation-20261008/`, retaining each complete
branch suffix. No implementation ref moved. Created and verified
`.scratch/ci-propagation-before-20261008.bundle` (5.4MiB), SHA-256
`256d3b1be04bd4e246f691a90e6fc9bed32a6030334c5cdc36dfffeb28301954`.
Its75 heads match the preview manifest exactly. Restored the bundle into
separate bare repository `.scratch/ci-propagation-restore-20261008`; full
`git fsck` passed. The earlier original #212/#217/unpublished backups remain
unchanged. No source files, migrations, PR bases or remote branches changed.

Next: finish composition14504 and remote402/404. When the prerequisite base
is verified, use the saved expected trees and recovery refs for the bounded
refresh, with rebase.updateRefs disabled and explicit remote leases. Revalidate
all heads before writing; a recovery checkpoint is not permission to overwrite
subsequent user changes or a substitute for final-head tests.

### Complete composition with CI corrections passed — 2026-10-08

The preceding iteration made progress by creating and independently restoring
the75-branch recovery checkpoint, published in ledgerccc827cb. Reconfirmed
local14504 and remote402/404 live, then waited without restarting any check.
Collected14504's terminal exit0 on clean604c0481770f1264eb07638518635e62dc991dd8:
the log ends in all checks passed and explicitly omits ARM/Darwin.

The complete composition passed191 core tests,1478 server tests with11 existing
manual measurements ignored, and all15 auxiliary binaries. The server suite
took356.90s with the two-thread budget. CLI framing, denied-export rollback and
the previously failing CSV interruption cases are included. Browser, Clippy,
SQLx and unchanged build outputs were reused where their derivations matched;
the changed tests and VM checks executed. Deployment/recovery finished91.22s
and OIDC29.07s, both with the explicit store-registration guard. Expected
database-creation denials are assertions within the successful VM script, not
failed build results.

Evidence remains `.scratch/permission-delivery-ci-604c0481-full-check.log` and
the isolated clean worktree. Its source difference from already-verified
f90f60f7 is exactly the three Nix files previously recorded; no Rust, SQLx,
browser assertions or migration changes were required. This closes the local
cross-extraction CI-configuration check, not new-head individual remote CI or
the remaining ARM VM gates. Original code and prior verification remain intact.

Next: collect402/404 ARM VM results before propagating the base. If those fail,
inspect their actual terminal logs rather than attributing every failure to
the previous TCG timeout. The75-branch preview/recovery checkpoint is ready;
heads and leases must still be revalidated before any rewrite or publication.

### Cancelled-worker session-release race — 2026-10-08

The preceding user-facing estimate was a status-only turn, not additional
verification progress. Revalidated the goal, #216's merged state, clean candidate
worktrees and live remote402/404 before continuing. ARM402 is still building the
VM image, not reporting a completed guest boot result.

The current-head CI inventory exposed another failure on #228951baf9d:
[ARM build338](https://nixbot.numtide.com/repos/github/numtide/horae/builds/338/logs/raw/checks.aarch64-linux.tests)
finished835 passed,1 failed,11 ignored. The page-consumer cancellation test's
single retry returned Busy. Traced the externally aborted future through
streaming::run_inner, the worker's shared session ownership and SQLx's
close-on-drop task. Pool capacity becoming available does not acknowledge
PostgreSQL's release of the old session lock. #224's explicit release_import
cleanup is not called on this externally aborted path; adding that dependency
would not fix the test's synchronization assumption.

Published draft #286, headed286e6b4d72687963290dda7b51242313886cd2, over #285
in isolated .worktrees/import-cancellation-release. Moved the existing CSV
wait_for_session_release helper to its parent test module and reused it in both
cancelled-worker tests. Reviewed that all four observer callers remain bounded
by existing five-second deadlines; Busy is retried only while observing cleanup,
other errors fail immediately. Busy-while-worker-active, actual single import
retry, empty clients/time entries, unchanged watermark and one-connection pool
assertions remain intact. No production retry, SQL change, migration, sleep,
timeout increase or new dependency was introduced.

Formatting and diff checks passed. Full native flake check91786 is running on
the published head; log .scratch/import-cancellation-release-ed286e6b-full-check.log.
Nixbot413 evaluation passed and its build is live. These are pending checks,
not a native/ARM pass claim. The failure above is the original CI regression
evidence, not a claimed deterministic local reproduction.

Next: collect91786 and413, plus402/404's ARM VM results. Once verified, update
the propagation preview to include #286 and verify its composition with #224;
the previous75-branch backup remains valid for unchanged extraction heads.
No original branch, feature branch, PR base or real data changed this iteration.

Prepared the full composition as8fc3a44e in the existing isolated CI worktree,
preserving604c0481 under backup/permission-delivery-ci-604c0481. The cherry-pick
had one insertion conflict with the existing imported-task archival test;
retained that entire test and the shared observer. Comparing every added/deleted
line against #286's standalone patch passed exactly (28 additions,25 deletions,
the same three test files). #224's transaction-cleanup code and regressions are
unchanged. Full composition check85532 is running; log
.scratch/permission-delivery-ci-8fc3a44e-full-check.log. This source composition
proof does not replace its pending test results. Collect the same live handles
instead of restarting either build.

### Complete CI propagation preview including cancellation synchronization — 2026-10-08

The preceding iteration made progress by publishing #286 and preparing its full
composition; ledgercef63082 is published. Revalidated both live local handles
91786/85532 and remote402/404/413. Native formatting passed in both worktrees;
both app builds have completed their web bundles and continue running. Remote
checks are still pending, not successful or stopped. No build was restarted.

Updated the tree-only propagation proof for target `ed286e6b`. Revalidated all75
local/remote heads, clean worktrees and saved recovery references. All75 trees
contain exactly the shared correction's edits in eight files, with no feature,
SQLx or migration edits. All57 code-PR review patches retain identical added
and removed lines and paths; unlike the prior #285 preview, this comparison
ignores hunk offsets and blob IDs because #286 also touches test files extended
by the task-feature PRs.

There are six insertion conflicts: #276–#280 and their existing
integration/task-catalog-prerequisites review base. Each inserts the archived
project-task regression at the same former gap as #286's helper. Preserved the
entire existing regression followed by the unchanged helper, matching the
already-reviewed full composition. The verifier accepts only this exact
empty-ancestor insertion conflict and fails on other conflicts or altered edits.
No branch, PR base, source worktree or remote ref was rewritten by this preview.

Evidence: .scratch/permission-ci-cancellation-preview-20261008.json and
.scratch/preview-ci-cancellation-propagation.mjs. The existing75-head recovery
bundle still matches every extraction head. #218/#248 are documentation-only
and remain outside code propagation; originals #212/#217/#208 remain excluded.

Next: collect91786/85532 and the exact-head ARM VM/test results. If the shared
base passes, revalidate leases and apply the refreshed preview in dependency
order, preserving the six explicitly reviewed insertion resolutions. Do not
treat the simulation as executed rebases or fresh-head CI acceptance.

The current GitHub base query also confirms11 extraction roots still review
against #282: #219, #220, #223, #224, #225, #227, #231, #238, #239, #240 and
#242. When publishing verified refreshed heads, retarget these11 to
test/import-cancellation-release (#286), otherwise their visible review diffs
would include shared CI corrections. All other extraction review-base names
remain unchanged. Do not retarget #283, whose real prerequisite remains #282,
or change the shared-fix chain. This is a pending publication step, not an
already-applied base change.

### Prepared bounded propagation and first ARM VM acceptance — 2026-10-08

The preceding iteration made progress through the complete #286 propagation
preview and recorded root-base updates, published ascc7e961f. This iteration
kept the same91786/85532 builds running. #286's native Clippy passed; browser
checks continue. The full composition has completed its app build and started
Clippy. Neither full gate has a terminal result yet.

Adapted the existing bounded rebase procedure into the private
.scratch/ci-cancellation-propagation.mjs with a scoped post-rewrite hook. Syntax
checks and read-only audit passed:75 exact local/remote heads, clean worktrees,
matching backups, zero rewritten branches. It disables rebase.updateRefs,
reuses the shared-commit mapping, verifies each resulting tree against the new
preview and stops at conflicts. Additional guards reject switched worktrees or
a changed shared target; the continuation mode retains the rewrite hook after
manual conflict resolution. No run-one/continue/publication operation was run.
Its audit is not evidence that the future rebases or their tests already passed.

Nixbot402's ARM deployment VM reached Horae listening at guest289s and proceeded
through import restart checks; it is no longer merely constructing the image or
waiting for the guest shell. Separately, the exact-head ARM OIDC test finished
successfully in444.68s; the log ends with successful artifact upload and the
succeeded attribute endpoint confirms checks.aarch64-linux.e2e-oidc. Its total
build duration was32m31s, including dependencies, distinct from test runtime.
Evidence: .scratch/nixbot-402-arm-oidc-summary.json and its public source URLs.
The ARM deployment/recovery result, full402/404/413 builds and both local gates
remain pending. No timeout or assertion was changed to obtain this result.

Next: collect those existing handles. After the prerequisite base and composition
are verified, use the prepared one-branch-at-a-time procedure, verify each exact
tree and preserve review-base bindings before publication. Do not restart builds
or rewrite the extraction branches while their shared candidate remains pending.

### Complete remote ARM VM correction accepted — 2026-10-08

The preceding iteration made progress by preparing the guarded propagation
procedure and recording ARM OIDC acceptance in published ledgerbbff47ce.
Continued verified waits on the same91786/85532 and402/413 handles; did not
restart builds or publish rewritten extraction heads.

Nixbot402's ARM deployment/recovery test completed successfully in993.95s,
including graceful termination, forced termination, checkpoint recovery and
repeated-import assertions. Its total runtime exceeding900s is not a guest
readiness timeout: guest startup had already passed and the functional scenarios
ran afterward. The succeeded attribute endpoint confirms both ARM VM checks,
both x86 VM checks and formatting. GitHub reports the complete402 nix-build
SUCCESS at2026-10-07 23:53:50 UTC for #28493b3fbaa; nix-eval also passed.
Evidence: .scratch/nixbot-402-complete-summary.json and its source URLs. Updated
#284's description with exact-head acceptance, without claiming a benchmark,
downstream CI success or permission to merge.

Locally, #286's full browser check and SQLx cache verification passed; its core
tests finished and the server test binary is compiling. The full composition
passed Clippy and continues its browser matrix. Both full gates remain live.
Nixbot404 and413 are not yet accepted as complete. The propagation preflight,
backups and original source branches remain unchanged.

Next: collect the remaining exact-head checks and complete-composition result,
then begin guarded propagation only when the shared base is verified. No merge
or original-branch modification occurred.

### Local staged rebases and complete native cancellation verification — 2026-10-08

The preceding iteration made progress through complete #284 remote acceptance,
published in ledger41a7ce7f. Reconfirmed #216 merged and remote404/413 live.
Adjusted the earlier operational hold to allow reversible local staging while
remote CI queues, not publication: the backup refs/bundle are verified, every
expected tree was precomputed, and neither running local gate uses an extraction
worktree. Remote heads and PR bases remain unchanged until prerequisites pass.
This keeps the original scope and publication gate intact without treating a
local rebase as test acceptance.

The guarded procedure has staged41/75 branches at this checkpoint, preserving
shared ancestry through the rewrite map and checking each resulting tree exactly
against the preview. The next bounded batch is live; every operation disables
rebase.updateRefs and unsigned commits are retained. Receipts are in
.scratch/ci-cancellation-propagation-completed.jsonl. The final review-diff and
remote-lease verifier is prepared but will run only when all75 are staged.

Rechecked original preservation: master, #212, #217 and #208 retain their
original heads. #212 still has the same18 dirty paths. Its tracked worktree is
identical to snapshotd364270a; tar comparison verifies the six untracked files.
The original unstaged patch SHA matchesd90b1797 when core.abbrev=7 is used,
matching the saved patch format; Git's now-longer automatic object abbreviation
changes patch text, not source content. The staged patch remains empty.

Collected native check91786 exit0 on clean #286ed286e6b. All121 core tests,
821 server tests with11 existing ignored measurements and nine auxiliary test
binaries passed. Both changed cancellation tests passed. Browser, Clippy, SQLx,
deployment/recovery82.10s and OIDC26.04s passed; the final log says all checks
passed and explicitly omits ARM/Darwin. Evidence remains
.scratch/import-cancellation-release-ed286e6b-full-check.log. Updated #286's
verification without claiming pending remote413 or composition85532 passed.

Next: finish local staging, handle only the six previously reviewed insertion
conflicts, then run the full tree/ancestry/review-patch audit. Continue collecting
85532 and remote404/413. Do not push rewritten extraction heads or retarget PRs
until their shared prerequisite checks are accepted. Originals remain excluded.

### Complete local propagation verified — 2026-10-08

The preceding status-only turn did not advance authoritative state. Revalidated
the stopped task-lifecycle rebase and the live composition handle85532, then
completed the six remaining local branches. Reconfirmed #216 merged before
continuing. The only stopped conflict was the previously previewed insertion in
engine_tests.rs: retained the complete task-archival test followed by the unchanged
session-release helper. The remaining branches reused the recorded commit mapping.
No original branch, running test worktree, remote extraction head or PR base changed.

The final verifier passed all75 exact expected trees, clean worktrees, shared-base
ancestry and backup references. It also checked all57 code-review patches against
their original merge bases: added/removed content and file modes are unchanged;
offsets and blob identifiers are deliberately not treated as source differences.
The original #208/#212/#217 local and remote heads are unchanged, and all75 remote
leases still match their pre-rebase values. Evidence:
.scratch/ci-cancellation-propagation-verified.json and the per-branch completed
receipts. This verifies preservation, not fresh-head test acceptance.

GitHub now reports the complete Nixbot404 aggregate SUCCESS for #28515d9ab5b,
completed at2026-10-08 00:06:55 UTC. #286ed286e6b still has Nixbot413 in progress
with evaluation passed. Composition85532 is running server tests, including both
cancelled-worker tests already passed; its full gate has not yet completed.

Next: collect the existing413/85532 handles. After prerequisite acceptance,
publish the verified75 branches with explicit remote leases and atomic push,
retarget the11 extraction roots from #282 to #286, and check the new heads.
The roots are #219/#220/#223/#224/#225/#227/#231/#238/#239/#240/#242; the shared
#282→#283→#284→#285→#286 chain remains unchanged. Keep original PRs and retained
unfinished work intact; no merge or policy activation is authorized by this step.

### Publication guards prepared while acceptance runs — 2026-10-08

The preceding iteration made progress: all75 local rebases and57 review patches
were verified and recorded in published ledgera6a410f3. The existing85532 and413
handles remain live; neither was restarted. Build413 is now running ARM tests.
The complete native composition has finished its Rust test phase and is building
the VM dependencies; no full-gate pass is claimed yet.

Prepared .scratch/publish-ci-cancellation-propagation.mjs using the saved
manifest and final verifier. Syntax and read-only audit passed for all75 branches
and57 still-open draft PRs. Its audit confirms original remote heads, unchanged
review bases and both acceptance gates pending. Publication requires #286's exact
head to pass both Nixbot checks and a collected exit0 receipt for85532 on unchanged
composition8fc3a44e. It rechecks patch identity, clean worktrees, original heads,
backups and remote leases, then uses an atomic push with explicit per-ref leases.
Retargeting is a separate resumable step allowed only after all75 remote heads
match the verified publication. No publish or retarget mode was executed.

Fresh comparisons also confirm the original #212 tracked worktree still matches
snapshotd364270a and its six untracked files still match the saved tar archive.
Next: collect85532 and413 without restarting; record exact-head acceptance before
publication, then verify the remote heads, the11 review-base changes and new CI.

### Complete native composition accepted — 2026-10-08

Collected the original85532 handle with exit0, not a replacement execution.
All compatible checks passed on clean composition8fc3a44e. The1478 server tests
and15 auxiliary binaries passed, with only the11 existing ignored measurements;
both cancellation cases passed. Browser, Clippy and SQLx passed, and the final
OIDC/deployment VM scripts completed in28.24s/67.24s. The final flake result
explicitly omitted ARM/Darwin, so this is not their acceptance or acceptance of
each unpublished extraction. Saved the exact head, session, exit status, counts
and SHA256 of the complete log in the publication receipt.

Remote #286 Nixbot413 still reports build IN_PROGRESS and evaluation SUCCESS.
The publication gate therefore remains closed only on remote prerequisite CI;
the complete native composition prerequisite is now satisfied. Next: collect413,
then run the prepared guarded publication and retargeting procedure, followed by
new-head CI and final delivery audit. Do not rerun85532 or claim old extraction
checks apply to their new commits.

### Refreshed coverage bridge and ARM cancellation acceptance — 2026-10-08

The preceding iteration completed the native composition gate and recorded it in
published ledger50bd30ef. Continued the same remote413 execution without a rerun.
Its ARM tests now succeeded:821 server tests with zero failures and11 existing
ignored measurements in88.12s, plus all nine auxiliary binaries. Both corrected
HTTP-waiter and page-consumer cancellation tests passed. Four filtered child
process summaries are not extra suites. The succeeded endpoint also confirms ARM
package, Clippy, SQLx and formatting. Full Nixbot413 remains IN_PROGRESS; browser
and VM acceptance are not inferred from these results. Evidence and source URLs:
.scratch/nixbot-413-arm-tests-summary.json.

Ran .scratch/verify-delivery-coverage-bridge.mjs successfully. It connects the
existing60-input content audit atf90f60f7 to the passed composition8fc3a44e: their
entire normalized edit patch matches exactly the shared #283→#286 correction
patch across six files. All57 old input heads match the preserved pre-rebase
manifest, all57 new review edit patches are unchanged, and every new head/tree
matches its verified receipt. #248's specification head is unchanged. Both the
browser runner and HTTP-matrix registration files are byte-identical to the
previously audited composition, preserving its34/25 registration coverage.

This is a preservation bridge, not another semantic review or proof that the
rebased commits are ancestors of the older composition. The prior seven explicit
query/fixture reconciliations remain recorded; no new production-source resolution
is introduced by the six-file correction. Saved the proof in
.scratch/permission-delivery-coverage-bridge.json and made it an additional
pre-publication guard. No source, migration, product contract or original ref was
modified. No claim of full-feature acceptance is added.

Next: wait for the complete413 prerequisite, publish the guarded75-branch update,
retarget the11 roots and verify fresh-head CI. Local85532 is finished and must not
be restarted. Keep incomplete acceptance and original #208/#212/#217 untouched.

### Independent refreshed-root checks started — 2026-10-08

The preceding iteration made progress through the coverage bridge and ARM test
acceptance, published in ledger2f4c2e51. Reconfirmed #286's exact-head build413
IN_PROGRESS. Its tests/package/Clippy/SQLx remain succeeded. The formatting
attribute, previously returned in the succeeded group, now appears only in the
building group and its raw log only announces the derivation. Recorded this
state correction without guessing a cause, requesting a retry or claiming a
completed gate. The full aggregate remains the publication prerequisite.

Advanced independent fresh-head verification while remote CI continues. No local
Nix/Cargo build was running;82GiB disk and56GiB available RAM were observed.
Started two bounded full native checks, each --max-jobs1 --cores2, on clean frozen
worktrees: #220ea78c27c in session67402 and #219690cce20 in session16392.
Both passed formatting and are compiling the app. Their logs are
.scratch/pr220-ea78c27c-full-check.log and .scratch/pr219-690cce20-full-check.log.
They certify distinct refreshed extraction heads only if their full checks pass;
neither duplicates the completed composition or #286's standalone check.

Next: collect those two existing handles and remote413. Do not edit their
worktrees, start replacement builds or treat source-preservation receipts as CI
success. Publication and the11 root-base changes still await remote prerequisite
acceptance; original branches remain excluded and no merge is requested.

### Exact check identity verified during root compilation — 2026-10-08

The preceding iteration started independent checks and recorded their handles in
published ledgerbc5174bc. Continued verified waits on67402/16392 and remote413.
Both local client builds completed; the two server rustc processes are active at
approximately183–184% CPU after about four minutes, consistent with their two-core
budgets. No terminal result or stalled-build inference is made from silent logs.

The GitHub commit-specific check-runs API confirms both #286 check runs have
head_sha ed286e6b4d72687963290dda7b51242313886cd2 and point to build413. Evaluation
is completed/success; build remains in_progress. Updated the private publication
guard to inspect the newest check run by ID for each required name and require
its exact head, completed state and success conclusion. This rejects an older
successful attempt when a newer check is pending or failed. Syntax and read-only
audit passed:75 local branches,57 PRs, original remote heads, accepted local
composition and remote acceptance still false. No publication was attempted.

Next: collect the same three live handles. Do not restart silent compilations,
modify the two test worktrees or use earlier extraction checks as new-head proof.

### Two refreshed roots accepted natively — 2026-10-08

The preceding iteration recorded the exact-head publication guard in published
ledger3c305db0. Continued verified waits on the same67402/16392/413 executions,
without replacements or source edits. Collected local16392 and67402 exit0 on
unchanged clean worktrees: #219690cce20 and #220ea78c27c. Both complete native
flakes passed, including application build, all-target Clippy, browser matrices,
live-schema SQLx cache verification, tests and deployment/OIDC VMs.

#219 passed158 nextest core tests,821 server tests and nine auxiliary binaries;
VM scripts took75.10s/17.82s. #220 passed121 core tests,828 server tests and nine
auxiliary binaries; VMs took66.00s/17.31s. Each retains11 existing ignored manual
measurements. Four filtered child-process summaries are not additional suites.
Saved exact heads, session exit codes and full-log hashes in
.scratch/refreshed-root-native-acceptance.json. Both logs explicitly omit ARM and
Darwin, so these are fresh native results, not remote or all-platform acceptance.

Remote413 remains in_progress with evaluation success. Its latest active work
includes ARM browser/deployment/OIDC and x86 browser/deployment/OIDC/tests; no
failed attribute was returned in the observed failed group. This is a verified
wait, not a reason to restart CI or publish before prerequisite acceptance.

Also checked progress-publication overhead: each ledger push opens Nixbot CI
(current426), while the preceding425 was cancelled. The package source filter
excludes .md, so this does not prove redundant Rust recompilation; it does add
evaluation/format activity. Keep incremental records in local unsigned commits
and publish them at substantive delivery checkpoints instead of every status
update. No CI was cancelled manually and no CI configuration changed.

Next: collect413; the two root native handles are terminal and must not be polled
or restarted. Their exact heads remain eligible for guarded publication once
#286 is accepted, alongside the other staged branches. Continue fresh-head
verification without modifying any worktree that has a live check.

After both roots completed, started the next two independent native checks on
clean exact heads: #22368661980, session12382, and #224af4656d7, session61314.
Each uses --max-jobs1 --cores2. Logs are .scratch/pr223-68661980-full-check.log
and .scratch/pr224-af4656d7-full-check.log. Observed94GiB free before starting;
only these two local checks are now live. No previous check was restarted.
Next action includes collecting these handles alongside413; do not edit their
worktrees while they run. This checkpoint is committed locally, not pushed.

### Documentation CI gap scoped independently — 2026-10-08

The preceding iteration completed native #219/#220 verification and started
#223/#224, saved in local commitsca4de908/879ed925. The same12382/61314 checks
remain live; both client builds passed. Remote413 is still in progress, with
ARM browser and VMs plus x86 browser/tests active at the last observation.

Reviewed first-delivery PR descriptions and #248's current checks while those
builds run. #248b6e13979 still has failed Nixbot216 ARM VMs despite successful
GitHub Flake Check/Format. Direct logs distinguish the causes: OIDC timed out
waiting900s for the shell connection, whereas deployment got through startup
and timed out after92.49s waiting for the repeated job's succeeded status under
the90s bound. Do not collapse the latter into a boot failure or claim either is
resolved on #248 without refreshed-head verification.

Prepared separate documentation tree previews, not branch rewrites. #248's three
commits touch only56 Markdown files. Merging its current head with shared #286
produces treec811b935 with no conflict; the full-index binary review patch is
byte-identical before/after, SHA256681c919ae5b75e7edd0f4c0b565d0225b43ce721ad733cd9897e904350d73886.
#218 at checkpoint879ed925 also previews cleanly and touches only the ledger.
Its head will change as progress is recorded, so recompute its final preview
before rebasing. Evidence: .scratch/documentation-ci-refresh-preview.json.

Keep the verified75-code/review-base publication distinct. After that publication,
refresh #248 and #218 onto #286 with their own backups, exact patch checks,
explicit remote leases and review-base changes. The existing code coverage bridge
requires #248's old head until the code publication and must not be invalidated
prematurely. The11 code-root retargets stay unchanged; the two documentation bases
are additional follow-ups, not new features or newly split deliveries. Preserve
all54 original feature documents, New Project transition rules and cache guidance.

Next: collect12382/61314 and413; publish code only after prerequisite acceptance,
then complete the two documentation refreshes and reconcile PR descriptions with
their actual published heads and check results. No merges or original-ref edits.

Continued verified waits after that checkpoint: both12382/61314 completed their
application builds and entered Clippy. Remote413 now reports only the two ARM VMs
building; browser and x86 tests have advanced out of the building group. Both VM
logs show successful register-nix-paths.service and continuing guest startup
around130–140s, not a terminal result. Keep waiting on the same executions without
raising timeouts or modifying tested worktrees.

### Shared prerequisite accepted and code branches published — 2026-10-08

The preceding iteration scoped the documentation follow-ups and recorded active
checks in local95fbde08/d61997ac. Used a single gh-pr-checks watcher87658 at300s
intervals for #286 while continuing the existing local12382/61314 checks.
Collected all three handles with exit0; none was replaced or restarted.

The GitHub commit-specific API confirms #286ed286e6b completed both required
Nixbot checks successfully, aggregate413 at2026-10-08 01:07:21 UTC. ARM VM logs
end in successful artifact upload: deployment/recovery947.88s and OIDC397.54s.
Evidence: .scratch/nixbot-413-complete-summary.json and the public413 URLs.
Updated #286's PR verification, retaining draft state and no-merge instructions.

#22368661980 and #224af4656d7 both completed the full native gate on unchanged
clean worktrees. Counts are recorded above and in
.scratch/refreshed-root-native-acceptance.json, with session IDs and full-log
SHA256. Deployment/OIDC took76.00s/19.22s for #223 and65.85s/27.69s for #224.
Neither result claims remote ARM acceptance for those extraction heads.

Publication preflight confirmed75 exact local trees,57 draft PRs with unchanged
old bases/heads, matching remote leases and protected original references, plus
the accepted shared base and native composition. Re-ran the full patch/ancestry
verifier and coverage bridge. Atomic push35748 exited0 and updated all75 existing
branches with explicit force-with-lease guards; no original branch was included.
The private publication receipt records all exact heads. Retarget4269 exited0
after updating #219/#220/#223/#224/#225/#227/#231/#238/#239/#240/#242 to #286.
Other review-base names and the #282→#283→#284→#285→#286 chain are unchanged.

Next: collect the post-publication verification, refresh #248/#218 separately
with recoverable backups and exact documentation patch checks, and reconcile
the PR descriptions and fresh-head CI. The code publication is not completion
of final verification. Original #208/#212/#217 remain untouched; no merge or
policy activation occurred. The two import-root native handles and watcher87658
are now terminal and must not be polled again.

### Documentation branches refreshed without content loss — 2026-10-08

Post-publication verification37949 exited0: all75 remote heads and all57 review
bases match the accepted plan, including the11 root retargets. The originals are
unchanged. Prepared separate documentation backups before moving either branch:
backup/docs-ci-propagation-20261008/permission-specification atb6e13979 and
backup/docs-ci-propagation-20261008/permission-pr-separation atc349b112, including
the locally recorded progress since the last push. Bundle
.scratch/documentation-before-ci-refresh-20261008.bundle verifies as complete,
SHA25661bd8494063b2f1fff0d57d89b452fc2a08c42092fb90c664b3bf82728d2545e.

Rebased #248's three commits and #218's195 commits onto #286 without conflicts,
with unsigned commits and rebase.updateRefs explicitly disabled. #248 is now
a6d2e091 with expected treec811b935. The ledger rebase head6da072ee has expected
tree37267f21. Both clean worktrees and full-index binary review diffs matched
their backups exactly. Formatting passed for both. This subsequent log entry
adds progress only; it does not alter the preserved specification patch.

Next: publish these two documentation branches with exact leases, retarget them
to #286, and check their new-head CI. Update PR descriptions to distinguish the
new publication from historical checks, then finish the remaining exact-head
verification and final delivery audit. No merge or full-feature completion claim.

### Published documentation and current-head review metadata — 2026-10-08

The preceding user-facing estimate was a status-only turn, not implementation
progress. Re-read the goal and repository rules before continuing. The pending
inventory tool failed because its approval review timed out; it did not produce
a remote snapshot. A subsequent read-only GitHub query succeeded and saved
.scratch/permission-delivery-post-propagation-20261008.json. This snapshot is
historical once another query or PR-body edit occurs, not a live CI feed.

Documentation publication59951 and retarget43966 both exited0. Remote #248
a6d2e091 and #218ff1192da are drafts based on test/import-cancellation-release.
The inventory confirms all57 code PR heads/bases match the accepted propagation
manifest; together with the two documentary and five shared-CI PRs, the delivery
inventory contains64 PRs. All57 code PRs had fresh Nixbot evaluation/build checks
in progress. This observation does not claim final acceptance or independently
attribute every build to its commit; exact-head check-run validation remains.

#218 GitHub run37712168908 attempt1 failed both jobs before tests: Hestia's
attestation lookup received HTTP503 from api.github.com. Verified the run is
terminal and belongs to ff1192da, then requested one failed-job retry98343,
exit0. Query44256 confirms attempt2 is in_progress on that same head. No workflow,
cache-verification rule, application code or test was changed. Nixbot484 remains
separate. #216 was reconfirmed merged at02f7b58acdcf126415f9ec89215da8cdada7d03f.

Updated and read back descriptions for #219/#220/#223/#224/#248/#218 in44537,
exit0. Each edit checked the old body, current head, base and draft flag before
writing, and verified the exact new body and unchanged metadata afterward.
The four code descriptions now cite their passed native full checks on the
published heads and link fresh Nixbot472/431/470/447 without claiming completion.
The documentary descriptions distinguish preserved patches and formatting from
pending full CI. Earlier-head test evidence is retained explicitly as historical.

Next: finish the same metadata refresh for the seven remaining root PRs and the
dependent extractions, collect the existing #218 retry, and validate fresh-head
CI before the final ownership/readiness audit. Keep the ledger progress local
until the next substantive publication checkpoint to avoid needless CI churn.
No original source branch, runtime data, feature scope or merge state changed.

Collected both follow-ups in the same iteration. Description update50279 exited0
after verifying #225/#227/#231/#238/#239/#240/#242. All11 direct code roots and
both documentary PR descriptions now name their published heads and #286 base;
the46 dependent code descriptions remain to refresh. Their original scopes and
review findings are retained; old test executions are explicitly historical.

GitHub query74535 confirms #218 run37712168908 attempt2 completed successfully
on unchanged ff1192da: Format at01:31:14 UTC and Flake Check at01:31:35 UTC.
The previous503 remains recorded. No second retry was needed. Updated #218's
description to distinguish this success from still-separate Nixbot acceptance.
Targeted ledger formatting94993 exited0 without changes. Next: refresh the46
dependent descriptions and correlate their fresh CI with exact published heads;
do not restart the now-completed #218 GitHub run.

### Exact-head CI audit and dependent descriptions — 2026-10-08

The preceding iteration made progress: thirteen descriptions were updated and
the unchanged #218 retry passed. The current read-only audit73290 completed for
all64 delivery PRs. Each commit-specific check-run response was checked against
the expected published head, with the newest run per check name selected.
The snapshot at01:35:31 UTC is saved in
.scratch/permission-delivery-ci-current.json: four accepted (#282/#284/#285/#286),
three failed (#228/#270/#283), and57 pending. #283's old VM failures are already
addressed by its downstream #284; this does not make #283's own red checks green.
The audit started no builds and changed no branches.

New failures are #22855382bd7/Nixbot427 and #270479cfe3f/Nixbot430. Downloaded
the package, Clippy and server-test logs from both terminal builds. Both Clippy
and test compilers exited on signal9/SIGKILL; both package compilers exited1
without a Rust E-code diagnostic in these logs. Each build's three logs name
one ARM builder, respectively elastic-arm-0b1d6a93 and elastic-arm-234f8218.
The logs do not establish OOM, a timeout, who sent SIGKILL, or a source-code
defect. No retry, timeout adjustment, assertion change or resource workaround
was made. Six complete logs and SHA256 values are recorded in
.scratch/nixbot-427-430-arm-failure-evidence.json. Infrastructure/resource
evidence or a controlled diagnostic reproduction is needed before choosing a fix.

Description batches50159/88531/16606 completed successfully: #221/#222/#226/#228,
#232–#237, #241/#243–#247, #249/#250/#253/#254. Each of these20 edits checked the
prior body, exact head/base and draft state, then read back and verified the
updated description. Removed superseded current-status sections, updated review
bases and prerequisite references, and kept source scopes and historical tests.
#228 explicitly reports the new failed CI rather than retaining old acceptance.
#270's equivalent update was also published and read back in52663, exit0, after
one approval-review timeout that executed nothing. Both current ARM failures are
now explicit in their PR descriptions. In total34 descriptions are refreshed
(32 code extractions and two documentary PRs);25 dependent descriptions remain.

Next: refresh the remaining25 dependent PR descriptions,
investigate the ARM compiler kills without assuming their cause, and complete
the exact-head gates and final ownership/readiness audit. No feature, migration,
runtime data, original branch or merge state was changed. Keep this progress
checkpoint local until substantive publication is useful.

### Description refresh completed and ARM diagnostic narrowed — 2026-10-08

The preceding iteration made concrete progress on metadata and exact-head CI.
Finished the remaining25 descriptions in batches30442/6420/35707, all exit0:
#255–#269, #271–#273 and #275–#281. Removed superseded current-status sections,
updated prerequisite hashes, retained feature limitations and labeled older
verification/conservation records as historical. No application source changed.
Final read-back31300 verified all59 expected bodies, published heads, review
bases and draft flags; evidence is
.scratch/published-description-verification-20261008.json.

CI audit77238 stopped with a GitHub connection reset after50 recorded responses.
Its terminal failure was collected before resuming. Continuation58833 retained
those exact-head observations, verified heads/bases again and queried only the14
missing results; it completed with64 records and no build restarts. The combined
01:50–01:58 UTC snapshot has four passed, three failed and57 pending, unchanged
in classification from the earlier complete snapshot. #248's GitHub Flake Check
and Format now both passed. The earlier complete snapshot remains saved as
.scratch/permission-delivery-ci-20261008-0135.json; the current file records the
later mixed-time observation rather than claiming simultaneous checks.

Read the pinned Crane setup hook at
/nix/store/44k6cm6lbjpfq3xnl9wg0jpqa5rpsvmf-source/lib/setupHooks/configureCargoCommonVarsHook.sh.
It already exports CARGO_BUILD_JOBS from NIX_BUILD_CORES when unset, and the
failure logs show that hook ran. The inspected repository package/check files
do not override that variable. This rules out assuming an absent Cargo job
budget; it does not establish the actual remote allocation or cause of SIGKILL.
The public Nixbot427 page exposes failure logs but no OOM/cgroup diagnostics.
Requested builder OOM/cgroup or scheduler logs for the two recorded ARM workers
around01:29 UTC. Continue independent audits while that evidence is unavailable;
no resource workaround, new dependency or blind retry has been introduced.

Added the current handoff at the beginning of this ledger and explicitly labeled
the prepublication verification checkpoint historical. Next: reconcile the
current documentation preservation proof with the existing content-coverage
bridge, finish final ownership/readiness evidence, and resolve the failed or
pending exact-head CI. The goal is not complete; no merge or original closure.

### Published-content preservation and unfinished-task audit — 2026-10-08

The preservation verifier completed successfully after explicitly accommodating
the already-published documentation rebase. It compares the old and current
full-index binary review patches, not merely their file lists. All 57 code
extraction patches are unchanged. Documentation #248 has 56 byte-identical
changed blobs and an identical review patch, SHA256
681c919ae5b75e7edd0f4c0b565d0225b43ce721ad733cd9897e904350d73886.
Its old merge base is 8b3cc257, and its current base is #286/ed286e6b.
Evidence: .scratch/permission-delivery-published-coverage-bridge.json.

The composed delivery's delta from the prior content audit equals exactly the
six-file shared-CI correction delta. The prior coverage accounting therefore
remains applicable: 1,159 exact single-variant paths, 108 shared paths, seven
explicitly reconciled paths, zero unaccounted paths and zero target-only paths.
This is preservation evidence, not a new semantic review or ARM acceptance.

The original scoped-permissions worktree still matches saved tracked snapshot
d364270a; tar comparison also passed for all six original untracked files.
An anchored task-checkbox audit found 238 unique IDs, 208 checked and 30
unchecked. Every unchecked ID occurs exactly once in the retained-work
classification, with no missing or extra IDs. Evidence:
.scratch/retained-task-classification-20261008.json. Source task state was not
changed, and checked source tasks are not treated as extraction acceptance.

Next: collect changes in the existing CI runs and resolve remaining exact-head
gates. Keep original branches, incomplete work and draft PRs intact. The last
user-facing estimate was a status-only turn, not additional completed work.

A single read-only GitHub rollup at 02:13 UTC revalidated all 64 published
heads, bases and draft flags. No check status, conclusion, URL or completion
timestamp changed from the last commit-specific audit; 57 checks remain live.
Evidence: .scratch/permission-delivery-ci-rollup-20261008.json. The first query
did not execute because approval review timed out; its one permitted retry
completed successfully. No builds were restarted. Ledger formatting passed
with zero changes before this observation was appended.

### Intermediate ARM failure and shared-CI delivery boundary — 2026-10-08

The preceding iteration completed preservation bookkeeping and verified live
checks. Inspected the shared-CI delivery boundary without modifying branches:
#284 includes all three commits since #282 (the two CSV-framing commits and
the VM store-image correction), totaling four files and 74 added lines. Its
existing exact-head build402 passes on both Linux architectures. In contrast,
#283's own build352 remains red. Corrected #283's obsolete description saying
that build was still in progress, then verified the updated body and unchanged
head. No acceptance was transferred from the descendant to the failed parent.

Requested authorization to consolidate the two corrections in #284, with #283
closed as superseded. This would preserve the three commits and all descendant
heads instead of forcing another rebase/publication cycle. No retarget, closure,
merge or consolidation has been performed while that answer is outstanding.

Read Nixbot's actual build pages, not only the aggregate GitHub check. At 02:15
UTC #220/build431 has 17 of 20 attributes done, one failed ARM tests attribute,
two running (ARM browser and OIDC), and one pending. #219/build472 has five
attributes done and 15 pending. Both builds are live; neither was restarted.
The failed #220 test compilation received signal9/SIGKILL on
elastic-arm-234f8218, the same builder named by #270/build430. Its complete raw
log contains no Rust E-code diagnostic. This reinforces the need for builder
diagnostics but does not prove memory exhaustion or the source of the kill.
Evidence: .scratch/nixbot-431-arm-test-failure-evidence.json, the raw log and
the two saved live-page HTML files. The aggregate CI snapshot remains historical
and is not silently relabeled as a new terminal result.

Next: retain the existing live jobs, obtain the requested infrastructure
diagnostics, and resolve the shared-CI consolidation choice. No new functionality,
policy activation, real data mutation, original branch change or merge occurred.

### Current index publication and intermediate-CI inventory — 2026-10-08

Published the documentation-only checkpoint on #218 as 092d25f4, fast-forward
from ff1192da. Verified its body, head, #286 base and draft state after the push.
Only specs/permission-pr-separation.md changed; no code branch was republished.
The current dependency index contains all 57 code extractions exactly once;
every listed dependency resolves and the graph is acyclic. Evidence:
.scratch/permission-current-dependency-index.json and
.scratch/pr218-delivery-index-publication.json. Formatting passed unchanged.

A 02:21 UTC GitHub comparison found no changed code-PR aggregate checks. Only
#218's new head changed: Nixbot486 evaluation passed and its build is running.
The old head's checks are not reused as acceptance of this commit.

Inspected all 57 live build pages once, verifying each page's linked commit
against its published head. All pages were retrieved; 15 already contain ARM
failures even though their aggregate checks remain in progress. At 02:22 UTC
#218/build486 had 18 reused attributes and two pending. Saved page observations
in .scratch/permission-live-attribute-audit.json rather than changing terminal
GitHub results in the older snapshot.

Collected 20 actual failure logs and their hashes. Fifteen show compiler
SIGKILLs; three explicitly report a farm timeout after 1,200 seconds (#240's
package, #269's package and #276's tests). #256's package compiler exited
without an exposed signal. #220's browser fixture reached invoice preparation
then timed out filling Period from after 30 seconds; even the subsequent body
snapshot timed out after three seconds. Earlier browser assertions passed and
the server log reports multi-second database statements, but these observations
do not establish the browser timeout's root cause. No timeout or assertion
was relaxed. Nine raw-log requests returned404 because their attributes were
dependency_failed; verified that status in the saved pages instead of treating
those as executed test failures.

Classification and complete evidence:
.scratch/permission-intermediate-failure-classification.json. Three ARM builders
are named across these logs: elastic-arm-0b1d6a93, elastic-arm-234f8218 and
elastic-arm-976bb8e2. The requested scheduler/cgroup diagnostics remain needed;
no source workaround, retry, cancellation or infrastructure mutation was made.
The prior 4–8 hour delivery estimate is no longer reliable given the expanded
failure inventory. Next: diagnose the builder/timeout evidence and the distinct
browser failure, while preserving live work and awaiting the consolidation
decision. The goal remains incomplete.

### Bounded upstream timeout investigation — 2026-10-08

The preceding iteration produced new attribute-level failure evidence, not just
a status poll. Read public upstream sources at identified commits without
modifying Horae or any infrastructure:

- Nixbot at 2474092c426a5656dc9493fa57558171acca2615:
  [instance settings](https://github.com/Mic92/nixbot/blob/2474092c426a5656dc9493fa57558171acca2615/nixbot/nixbot/config.py)
  default build_max_silent_time to1,200 seconds and build_timeout to10,800.
  [build_nix_command](https://github.com/Mic92/nixbot/blob/2474092c426a5656dc9493fa57558171acca2615/nixbot/nixbot/executor.py)
  passes the silence limit to nix build. Its separate process-group timeout is
  the wall-clock limit, not the same setting.
- The inspected
  [repository configuration schema](https://github.com/Mic92/nixbot/blob/2474092c426a5656dc9493fa57558171acca2615/nixbot/nixbot/repo_config.py)
  does not expose those instance settings. Adding guessed timeout keys to
  Horae's nixbot.toml is not a demonstrated fix.
- The gRPC backend at b049a5a23130dfe07fd0d749824350bb34acfaee
  [transmits both limits](https://github.com/Mic92/nix-grpc-store/blob/b049a5a23130dfe07fd0d749824350bb34acfaee/src/client/build.cc)
  and its
  [backend options](https://github.com/Mic92/nix-grpc-store/blob/b049a5a23130dfe07fd0d749824350bb34acfaee/src/daemon/backend.cc)
  pass them to Nix. The
  [farm documentation](https://github.com/Mic92/nix-grpc-store/blob/b049a5a23130dfe07fd0d749824350bb34acfaee/docs/farm.md)
  distinguishes concurrent-build maxJobs from resource protection of the Nix
  and RPC services.

The1,200-second silence default is consistent with the three observed farm
timeouts, but the deployed revision and effective configuration are unknown.
It is an investigation lead, not proof that this exact setting caused them.
It does not explain the15 compiler SIGKILLs or the browser timeout. Research
receipts and selected sources are saved in .scratch/upstream-nixbot-timeout-research.json
and .scratch/upstream-grpc-timeout-research.json.

The browser failure is at newFeeInvoice filling Period from. Its locator had
resolved the input, and the failure handler could not obtain a body snapshot
within three seconds. Reading the fixture confirms no missing selector in that
path; this still does not prove resource starvation or exclude an application
issue. Its x86 pass cannot substitute for ARM verification.

Next requires the requested builder diagnostics: deployed revisions, effective
silence/wall-clock limits, concurrent build allocation, and kernel/cgroup or
scheduler kill records for the three named workers. Keep SIGKILL, silence
timeout and browser failure as separate hypotheses. Do not extend timeouts,
emit artificial keepalive logs, lower assertions, or rerun the full matrix
without a supported diagnosis. Existing builds remain live; no new build,
retarget, consolidation or merge was requested in this investigation.

### Completion-blocker audit and handoff — 2026-10-08

The same external dependency has remained through more than three consecutive
goal iterations: builder/scheduler diagnostics are unavailable for terminal
ARM failures. Independent work was completed meanwhile: preservation and task
classification, publication of the complete delivery index, intermediate-build
failure classification, and bounded upstream research. The last investigation
was progress; this audit does not mistake a live job for a stopped process.

Revalidated #228 at55382bd7 and #270 at479cfe3f: Nixbot427/430 remain terminal
failures, with no replacement acceptance. #218 at092d25f4 still has live
Nixbot486. Original PRs remain out of scope for closure or merge. Pending tests
are not a reason to change or weaken the completion criteria.

| Completion requirement | Evidence and remaining gap |
| --- | --- |
| Original work inventoried and recoverable | Source accounting, saved refs/bundles, tracked snapshot and six-file archive verified |
| Every change owned or explicitly retained | Preserved coverage bridge: zero unaccounted/target-only paths; all 30 unchecked task IDs classified once |
| Bounded deliveries with dependency order | 57 code PRs and current acyclic index published; documentation and shared-CI increments separately identified |
| Source/adversarial and combined verification | Bounded review records retained; complete native composition passed; not a substitute for current-head remote acceptance |
| Required extraction gates | Not met: terminal ARM failures and intermediate failures remain, with other builds live |
| Incomplete work separated | Original refs and seven unpublished Clients documents retained; broad acceptance/cutover not claimed complete |
| No unauthorized feature, activation, real-data or original-PR changes | This diagnostic sequence changes only the existing ledger and PR descriptions; no infrastructure mutation or merge |
| Final ready delivery | Not achieved; drafts and failed/pending checks remain explicit |

Safe autonomous investigation has reached the available evidence boundary.
The next useful input is the operator's deployed revisions, effective build
concurrency and timeout settings, and kernel/cgroup/scheduler kill records for
the three named ARM builders around01:29–02:18 UTC. Use those to select and
test a bounded correction before any retry. The separate consolidation request
can remove the obsolete shared-CI delivery boundary but cannot fix ARM kills.
Preserve all live jobs and do not repeat unchanged checks as a substitute for
that missing evidence. No completion claim is made.
