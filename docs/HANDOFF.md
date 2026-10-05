# Fork handoff

Base is MartyPC's exact FluxFox dependency 5a1fb836; native algorithms stay intact.
Context PR1 is integrated on main 531a0f0 after Windows/Linux CI. Counters/alias
paths are saved separately from image bytes. Source-bound 65 serde/60 native
checks, four actual mutants and scoped review pass. Full evidence lives in:
https://github.com/janrysavy/pyro221_next/tree/master/docs/evidence/martypc_rpc_20261004

Source-map PR2 is WIP. Original 2c26243 passes 69 serde/60 native tests and four
actual mutants. Review requested nested schema refusal, precise empty/cursor
scope and independent candidate/up/sibling coverage. Follow-up adds all of these.
Capture is between host cursor calls, not an active borrowed cursor/call frame.
Native empty Default is preserved as storage-only; native last_node panics both
before and after restore rather than silently repairing the original state.
Cargo fresh=false means recreated (not cached); deletion/timestamps and Git-bound
source already proved product freshness. Follow-up Windows suite passes 70 tests.
Source-bound 70 serde/60 native tests and six actual omissions pass on 1ff99b1.
Follow-up review finds no tree implementation defect; requests an independent
pinned native baseline and an explicit real-owner dynamic-type assertion. Type
assertions and an unchanged 5a1fb836 native fixture are added. Native baseline
measures None/null/real ownership, hidden null cursor state, image JSON omission
and independent clone/tree continuation. Full trees are compared in tests.
Native decoded MFM weak bits also diverge after clone/JSON despite identical
serialized state; 512 disabled-weak data bits match. RNG remains OPEN.
Windows library suite passes 70 tests with the independent full-tree fixture.
All-input-bound 70/60 products and final Windows/Linux CI pass on 74d66a8.
Latest review found empty null owner acceptance, unpinned native fixture provenance
and snapshot-only strictness applied to ordinary serde. Empty null capture/restore
is now refused; actual omission fails the new test, exact restoration precedes
fresh source-bound 71 serde/60 native tests. The live owner stays unchanged.
Fixture provenance is now mechanically pinned (full canonical-LF fixture hash,
original product SHA-256 and independently checked native receipt). Actual
product-hash corruption passes the old test, fails after the pin; exact fixture
restoration precedes fresh all-input-bound71/60 products. Strict nested decoding
now belongs only to the snapshot format; native public serde derives retain the
original tolerance. A fresh unchanged5a1fb836 probe proves four unknown-field
locations/three nullable omissions; regression fails before the fix. All nine
actual omissions fail their named tests, exact restoration precedes fresh
all-input-bound72 serde/60 native products. Final scoped review/CI remain pending;
no completed media/controller/Machine/process restart is claimed.

Next: finish source-map final gates/review/CI and linear integration; then full
media payload/weak-bit entropy/external owners, FDC/drive state and Machine/process
restart. Do not assume complete media/controller/Machine snapshots exist.
