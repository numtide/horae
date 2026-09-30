# Record-scope evaluation contract

`AccessScope::covers(&Actor, &ScopedResource, &ManagementAssignments) -> bool` is a pure predicate, not an authorization endpoint.

All evaluations deny when the actor is inactive, the resource belongs to another organization, or assignment provenance does not match the actor and organization. Matching organization scope never bypasses those constraints.

After those checks, any explicitly granted scope may match:

| Scope | Match |
| --- | --- |
| None | Never |
| Own | Resource person is the actor |
| Managed people | Resource person occurs in trusted managed-person assignments |
| Managed projects | Resource project occurs in trusted managed-project assignments |
| Organization | Any same-organization resource |

Missing person/project IDs never match that dimension. An assignment without its corresponding scope grants nothing. Own access does not imply project management. Person and project scope combine by union, not intersection; matching both returns one boolean, not multiple records. Calling code must likewise avoid duplicate SQL rows/totals.

Tests cover all 16 combinations of scope flags across self, managed-person, managed-project, overlapping, unrelated and identity-less resources. They also cover inactive actors, foreign resources, foreign/misattributed assignment sets, missing IDs, no grants, removed assignments and union identity/idempotence/commutativity/associativity.

## Trust and concurrency boundary

Facts must be loaded from the database under the server's current authorization context. This function cannot detect forged membership IDs or stale snapshots. Re-evaluation with removed assignments denies access, but transactional revocation safety still requires the later persistence/service design. Capability dependencies, UI filters, state transitions and invoice/administrative locks are not implemented here and cannot be bypassed using this predicate.
