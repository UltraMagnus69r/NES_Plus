# UnMaker-first roadmap

Version: R0, 2026-11-17.

This sequence is separate from historical Phases 0–9.1. Those phases stay in git history and in [packaging/phase9/](../../packaging/phase9/) and [packaging/phase91/](../../packaging/phase91/). They are not renumbered. Historical Phases 10–20 stay suspended. Do not start Phase 10.

The owner approved this order: finish UnMaker Lab before more Maker Engine features. Maker behavior that already works stays. New Maker work waits for the integration gate.

R0 is this audit and these documents. It does not implement R1 or the Goomba slice.

| Step | Name | Done when |
| --- | --- | --- |
| R0 | Architecture rebaseline and code audit | The owner accepts the KEEP/REPAIR/REPLACE decision, the asset lifecycle, and the Goomba plan in this directory. |
| R1 | UnMaker studio foundation | One workspace shell can import a ROM, show investigation progress, and keep reconstruction state in the project file. No new visual theme yet unless the owner asks. |
| R2 | Verified NES extraction and automatic reconstruction | The Goomba slice in [GOOMBA_VERTICAL_SLICE.md](../architecture/GOOMBA_VERTICAL_SLICE.md) passes on the owner's ROM: animated preview, approve, save, reopen, ROM no longer required. |
| R3 | Intelligent issue resolution and local learning | One unresolved issue is presented with a suggestion. Accept, skip, and reject are stored. A rejected grouping is not offered again unchanged. |
| R4 | Reference intelligence and AI integration | A selected reference can sit beside the reconstruction. AI runs only with a credential and an explicit send. |
| R5 | Complete animation and creation studio | The user can edit frames, clips, forms, timing, origins, and anchors, and see the preview update. |
| R6 | Gameplay content, physics, and behavior authoring | An approved picture can receive one existing Blueprint and one behavior graph. No second physics model. |
| R7 | Permanent library, packaging, and Maker handoff | One package round-trips into Maker as a placeable animated object. |
| Gate | UnMaker-to-Maker integration acceptance | The owner plays that object in Maker from a reopened project. |
| After | Historical Maker roadmap | Phases 10–20 may resume. They do not start before the gate. |

Phase 9.1 remains unaccepted. Its test candidate does not satisfy R2.
