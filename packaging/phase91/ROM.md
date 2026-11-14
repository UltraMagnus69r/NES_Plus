# ROM extraction

Nesticles reads an iNES or NES 2.0 header, checks that the file is long enough for the declared PRG and CHR, and reports the mapper, mirroring, trainer bit, and both SHA-256 digests. The whole-file digest and the PRG+CHR payload digest are different. Identity uses the payload digest.

CHR-ROM tiles are decoded in place. The source file is not modified and is not written into the project.

A named reconstruction is applied only when the payload digest matches a profile. The shipped Super Mario Bros. (World) profile uses the public No-Intro headerless digest `fcb6a0ef3a20c19b356005fbb21dc8009563b1cb5a9aaebc8e9386b4a8c5912e`. Enemy tile indexes in that profile come from the public disassembly table. They are proposals, not approved art, and they are not pixels stored in Nesticles.

If your dump's payload digest differs, including a different region or a header-only mismatch that changes the payload, Nesticles keeps the raw tiles and does not rename them as Mario. Mario's player graphics are not in the shipped profile. That row stays unresolved until you assemble it.

A reference URL is optional. It must be HTTPS on port 443 and must not target a private or loopback address. Page scripts are not run. You can continue without a URL.

Do not commit a commercial ROM. The synthetic fixture in the home screen is original test data, not Super Mario Bros.
