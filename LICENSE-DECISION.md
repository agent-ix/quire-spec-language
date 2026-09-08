# License decision before implementation or publication

Private bootstrap, 2026-09-07.

The owner approved the proposed native finite-state implementation profile and
`AGPL-3.0-only` for new compiler implementation in the active task conversation
on 2026-09-07, replying: “yes ok lets go AGPL I leave the implementation order
up to you!” This followed the explicit proposal of `AGPL-3.0-only`.

New implementation source, build/check scripts and implementation test fixtures
use SPDX `AGPL-3.0-only`. The included LICENSE is the unmodified GNU Affero
General Public License version 3 text; source notices select version 3 only.
No third-party implementation or generated parser is copied. Existing imported
source grants and notices remain intact, including MIT/Apache contract and
temporal libraries. No such implementation dependency is adopted by LC01.

The standard's document/reusable-artifact license remains deferred. This record
does not license the separate specification repository, relicense bootstrap
history, approve public release or close the ecosystem artifact review.

Every release needs an included-file inventory, inbound rights, exact outbound license, dependency compatibility, notices and generated-content review. No implementation source is copied by this initialization. Repository visibility and license are separate decisions.
