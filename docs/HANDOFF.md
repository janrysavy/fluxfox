# Fork handoff

Base is MartyPC's exact FluxFox dependency 5a1fb836; native algorithms stay intact.
FINISHED LOCAL: context-only snapshot supplement preserves native write counters,
optional contexts and cross-image/track aliases, including nested decoded/resolved
flux caches. Strict graph preflight precedes rebinding candidates to fresh locks.
Windows library suite: 65 passed, 0 failed, including five context tests.

This is not complete media/controller/Machine state: source maps, weak-bit entropy,
external image owners and full media payload validation remain OPEN. Seeded flux
cache tests establish storage graph preservation, not native flux continuation.
Initial fixtures failed twice: native metasector creation increments the counter
before 17 explicit increments (18); JSON track payload uses FluxStreamTrack tag.
Both test expectations were corrected; the final complete suite passed.

First source-bound run on 9eb9877 passed 65 serde/60 native tests and four actual
omission controls. Review requested simultaneous context locking and stronger
mixed-owner/isolation evidence. Follow-up holds all distinct locks during counter
capture and tests aliases across images, unlike owner kinds and nested caches.
Follow-up source-bound gates on 2135382 pass 65 serde/60 native tests after
verified test-product deletion; all 297 tracked compilation inputs match Git
before/after the run. Four actual mutants fail their named tests; exact source
is restored and positive suites pass. Scoped follow-up review finds no defect.
Only integrate after Windows/Linux CI passes on this final head. Source maps,
weak-bit entropy, image payloads, controller and Machine/process stay OPEN.
