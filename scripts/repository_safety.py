#!/usr/bin/env python3
"""Reject staged or tracked content that looks like a ROM, a secret, or an unknown large binary.

This does not decide copyright ownership. It blocks file types and signatures that
this repository has already classified as local-only, and it fails closed on
archives and payloads it cannot explain. Synthetic fixtures are created in a
temporary directory by --self-test and are not committed.
"""

from __future__ import annotations

import argparse
import base64
import io
import os
import re
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROM_EXTENSIONS = {
    ".nes",
    ".nez",
    ".unf",
    ".unif",
    ".fds",
    ".nsf",
    ".nsfe",
    ".sfc",
    ".smc",
    ".gb",
    ".gbc",
    ".gba",
    ".nds",
    ".n64",
    ".z64",
    ".v64",
    ".sms",
    ".gg",
    ".32x",
    ".rom",
}

ARCHIVE_EXTENSIONS = {".zip", ".tar", ".gz", ".tgz", ".tzst"}

# Tracked files that are already in history and are not ROMs.
# A new large binary outside this list fails closed until a person reviews it.
KNOWN_LARGE = {
    "crates/nesticles/assets/icon.png",
    "crates/nesticles/assets/LiberationSerif-Bold.ttf",
    "crates/nesticles/assets/LiberationSerif-Regular.ttf",
    "crates/nesticles-player/pkg/nesticles_player_bg.wasm",
    "packaging/alpha1/Nesticles-Alpha1-linux-x86_64.tar.gz",
}

LARGE_BYTES = 2_000_000
SECRET_RE = re.compile(
    r"(sk-[A-Za-z0-9]{8,}|-----BEGIN (?:OPENSSH |RSA |EC )?PRIVATE KEY-----|AKIA[0-9A-Z]{16})"
)
B64_RE = re.compile(rb"[A-Za-z0-9+/]{64,}={0,2}")
INES = b"NES\x1a"
NSF = b"NESM\x1a"
UNIF = b"UNIF"
FDS = b"FDS\x1a"
GB_LOGO = bytes.fromhex("ceed6666cc0d000b03730083000c000d")


class Finding:
    def __init__(self, rule: str, path: str, detail: str) -> None:
        self.rule = rule
        self.path = path
        self.detail = detail

    def format(self) -> str:
        return f"{self.rule}: {self.path}: {self.detail}"


def repo_root() -> Path:
    out = subprocess.check_output(
        ["git", "rev-parse", "--show-toplevel"], text=True
    ).strip()
    return Path(out)


def git_bytes(args: list[str]) -> bytes:
    return subprocess.check_output(args)


def staged_paths() -> list[str]:
    raw = git_bytes(["git", "diff", "--cached", "--name-only", "-z", "--diff-filter=ACMR"])
    return [p.decode() for p in raw.split(b"\0") if p]


def tracked_paths() -> list[str]:
    raw = git_bytes(["git", "ls-files", "-z"])
    return [p.decode() for p in raw.split(b"\0") if p]


def read_worktree(root: Path, rel: str, limit: int | None = None) -> bytes:
    path = root / rel
    with path.open("rb") as handle:
        return handle.read() if limit is None else handle.read(limit)


def magic_kind(blob: bytes) -> str | None:
    if blob.startswith(INES):
        return "iNES header"
    if blob.startswith(NSF):
        return "NSF header"
    if blob.startswith(UNIF):
        return "UNIF header"
    if blob.startswith(FDS):
        return "FDS header"
    if len(blob) > 0x114 and blob[0x104:0x114] == GB_LOGO:
        return "Game Boy logo bytes"
    return None


def looks_like_container(blob: bytes) -> bool:
    return (
        blob.startswith(b"\x7fELF")
        or blob.startswith(b"\x89PNG")
        or blob.startswith(b"PK\x03\x04")
        or blob.startswith(b"\x1f\x8b")
        or blob.startswith(b"RIFF")
        or blob.startswith(b"OggS")
        or blob.startswith(b"\x00asm")
    )


def scan_bytes(rel: str, blob: bytes, findings: list[Finding]) -> None:
    suffix = Path(rel).suffix.lower()
    if suffix in ROM_EXTENSIONS:
        findings.append(
            Finding(
                "rom-extension",
                rel,
                f"{suffix} is a cartridge or ROM extension and stays off GitHub",
            )
        )
    kind = magic_kind(blob[:0x120] if len(blob) > 0x120 else blob)
    if kind and not looks_like_container(blob):
        findings.append(Finding("rom-signature", rel, kind))
    base = rel.split("@", 1)[0]
    if len(blob) >= LARGE_BYTES and base not in KNOWN_LARGE:
        findings.append(
            Finding(
                "large-binary",
                rel,
                f"{len(blob)} bytes is outside the reviewed large-file list",
            )
        )
    if not looks_like_container(blob):
        text_suffixes = {
            ".rs",
            ".md",
            ".json",
            ".txt",
            ".toml",
            ".yml",
            ".yaml",
            ".py",
            ".sh",
            ".html",
            ".css",
            ".js",
            ".nasset",
            ".nknowledge",
            ".env",
            ".mdc",
        }
        if suffix in text_suffixes or suffix == "":
            scan_textish(rel, blob, findings)
        else:
            scan_secrets_only(rel, blob, findings)
    if suffix in ARCHIVE_EXTENSIONS or blob.startswith(b"PK\x03\x04") or blob.startswith(b"\x1f\x8b"):
        scan_archive(rel, blob, findings)


def scan_secrets_only(rel: str, blob: bytes, findings: list[Finding]) -> None:
    if len(blob) > 2_000_000:
        return
    try:
        text = blob.decode("utf-8")
    except UnicodeDecodeError:
        return
    if SECRET_RE.search(text):
        findings.append(
            Finding(
                "secret-pattern",
                rel,
                "matched a key pattern; the value was not printed",
            )
        )


def scan_textish(rel: str, blob: bytes, findings: list[Finding]) -> None:
    if INES in blob or NSF in blob:
        findings.append(
            Finding(
                "embedded-signature",
                rel,
                "text or project file contains a raw NES or NSF header",
            )
        )
    try:
        text = blob.decode("utf-8")
    except UnicodeDecodeError:
        text = ""
    if SECRET_RE.search(text):
        findings.append(
            Finding(
                "secret-pattern",
                rel,
                "matched a key pattern; the value was not printed",
            )
        )
    if len(blob) > 2_000_000:
        return
    for match in B64_RE.finditer(blob):
        raw = match.group()
        try:
            decoded = base64.b64decode(raw, validate=False)
        except Exception:
            continue
        if decoded.startswith(INES) or decoded.startswith(NSF):
            findings.append(
                Finding(
                    "embedded-base64",
                    rel,
                    "base64 payload decodes to an NES or NSF header",
                )
            )
            return


def scan_archive(rel: str, blob: bytes, findings: list[Finding]) -> None:
    names: list[tuple[str, bytes]] = []
    try:
        if blob.startswith(b"PK\x03\x04") or rel.endswith(".zip"):
            with zipfile.ZipFile(io.BytesIO(blob)) as archive:
                for info in archive.infolist():
                    if info.is_dir():
                        continue
                    with archive.open(info) as member:
                        names.append((info.filename, member.read(512)))
        else:
            with tarfile.open(fileobj=io.BytesIO(blob), mode="r:*") as archive:
                for member in archive.getmembers():
                    if not member.isfile():
                        continue
                    extracted = archive.extractfile(member)
                    head = extracted.read(512) if extracted is not None else b""
                    names.append((member.name, head))
    except (tarfile.TarError, zipfile.BadZipFile, OSError) as err:
        findings.append(
            Finding("archive-unreadable", rel, f"could not inspect archive ({err})")
        )
        return
    for name, head in names:
        suffix = Path(name).suffix.lower()
        if suffix in ROM_EXTENSIONS:
            findings.append(
                Finding(
                    "archive-rom-name",
                    rel,
                    f"member {name} uses a ROM extension",
                )
            )
        if head.startswith(INES) or head.startswith(NSF) or head.startswith(FDS):
            findings.append(
                Finding(
                    "archive-rom-signature",
                    rel,
                    f"member {name} starts with a ROM header",
                )
            )


def scan_paths(root: Path, rels: list[str]) -> list[Finding]:
    findings: list[Finding] = []
    for rel in rels:
        path = root / rel
        if not path.is_file():
            continue
        scan_bytes(rel, path.read_bytes(), findings)
    return findings


def history_paths() -> list[tuple[str, str]]:
    raw = git_bytes(["git", "rev-list", "--all", "--objects"])
    found: list[tuple[str, str]] = []
    for line in raw.splitlines():
        parts = line.decode(errors="replace").split(" ", 1)
        if len(parts) != 2:
            continue
        digest, path = parts
        if len(digest) != 40:
            continue
        found.append((digest, path))
    return found


def scan_history() -> list[Finding]:
    """Read every reachable blob, not only filenames.

    ELF, PNG, gzip, and wasm containers still go through scan_bytes, which does
    not treat an iNES constant inside those containers as a ROM.
    """
    findings: list[Finding] = []
    seen: set[str] = set()
    for digest, path in history_paths():
        if digest in seen:
            continue
        seen.add(digest)
        try:
            kind = git_bytes(["git", "cat-file", "-t", digest]).decode().strip()
        except subprocess.CalledProcessError:
            findings.append(Finding("history-unreadable", path, digest[:12]))
            continue
        if kind != "blob":
            continue
        blob = git_bytes(["git", "cat-file", "blob", digest])
        label = f"{path}@{digest[:12]}"
        scan_bytes(label, blob, findings)
        if path in KNOWN_LARGE:
            findings[:] = [
                item
                for item in findings
                if not (item.rule == "large-binary" and item.path == label)
            ]
    return findings


def self_test() -> int:
    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="nesticles-safety-") as temp:
        root = Path(temp)

        def expect(name: str, data: bytes, should_fail: bool) -> None:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            found = []
            scan_bytes(name, data, found)
            failed = bool(found)
            if failed != should_fail:
                failures.append(
                    f"{name}: expected fail={should_fail}, got {[item.rule for item in found]}"
                )

        expect("ok.txt", b"Nesticles notes. No cartridge.\n", False)
        expect("renamed.dat", INES + b"\x00" * 12, True)
        expect("game.nes", b"not really", True)
        payload = base64.b64encode(INES + b"\x00" * 48)
        expect("embedded.json", b'{"sprite":"' + payload + b'"}', True)
        expect("notes.md", b"The iNES check looks for NES\\\\x1a in source.\n", False)
        expect(
            "secret.env",
            b"NESTICLES_OPENAI_API_KEY=" + b"sk-" + b"testsecretvalue\n",
            True,
        )
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            archive.writestr("readme.txt", "hello")
            archive.writestr("hidden.nes", INES + b"\x00" * 8)
        expect("bundle.zip", buffer.getvalue(), True)
        elf = b"\x7fELF" + INES + b"\x00" * 32
        expect("nesticles-binary", elf, False)
    if failures:
        print("SELF-TEST FAIL")
        for line in failures:
            print(line)
        return 1
    print("SELF-TEST PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--staged", action="store_true")
    parser.add_argument("--worktree", action="store_true")
    parser.add_argument("--history", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--files", nargs="*")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    root = repo_root()
    os.chdir(root)
    findings: list[Finding] = []
    if args.files:
        findings.extend(scan_paths(root, args.files))
    elif args.history:
        findings.extend(scan_history())
    else:
        rels = staged_paths() if args.staged or not args.worktree else tracked_paths()
        if args.worktree:
            rels = tracked_paths()
        findings.extend(scan_paths(root, rels))
    if findings:
        print(f"Repository safety: {len(findings)} finding(s).")
        for item in findings:
            print(item.format())
        print("Nothing was uploaded. Fix the files or leave them untracked.")
        return 1
    print("Repository safety: no findings.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
