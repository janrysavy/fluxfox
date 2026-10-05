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

Next: actual controls/fresh source-bound tests and scoped review, then final
Windows/Linux CI and linear integration before MartyPC dependency use.
