#!/usr/bin/env python3
"""Fetch and verify official GAEB DA XML references for local development."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tempfile
from urllib.request import Request, urlopen
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parents[1]
REFERENCES = ROOT / "references"
MANIFEST = REFERENCES / "SOURCE-MANIFEST.json"

SCHEMA_RELEASES = {
    "gaeb-da-xml-3.4-2026-03-beta": ["2026-03_BETA-Schemadateien.zip"],
    "gaeb-da-xml-3.3-2023-01": [
        "2021-05_Leistungsverzeichnis.zip",
        "2021-05_Handel.zip",
        "2021-05_Kosten_und_Kalkulation.zip",
        "2023-01_Mengenermittlung.zip",
        "2021-05_Rechnung.zip",
        "2021-05_Beta.zip",
        "2021-05_Zeitvertrag.zip",
    ],
    "gaeb-da-xml-3.3-2021-05": [
        "2021-05_Leistungsverzeichnis.zip",
        "2021-05_Handel.zip",
        "2021-05_Kosten_und_Kalkulation.zip",
        "2021-05_Mengenermittlung.zip",
        "2021-05_Rechnung.zip",
        "2021-05_Beta.zip",
        "2021-05_Zeitvertrag.zip",
    ],
    "gaeb-da-xml-3.2-2013-10": [
        "Leistungsverzeichnis.zip",
        "Handel.zip",
        "Kalkulation_3.2_2013-10.zip",
        "Mengenermittlung_3.2_2013-10.zip",
        "Rechnung_3.2_2013-10.zip",
        "Zeitvertrag_3.2_2014-03.zip",
    ],
    "gaeb-da-xml-3.1-2009-12": [
        "Schema_GAEB_DA_XML_3.1_2009-12_X81-X83_u_X85-X87.zip",
        "Schema_GAEB_DA_XML_3.1_2009-12_X84.zip",
    ],
    "gaeb-da-xml-3.1-2007-11": [
        "schema_gaeb-da-xml-3.1_20-11-2007_X81-X83_u_X85-X88.zip",
        "schema_gaeb-da-xml-3.1_20-11-2007_X84.zip",
        "schema_gaeb-da-xml-3.1_20-11-2007_X93_X94_X96_X97.zip",
    ],
}

EXAMPLES = {
    "gaeb-da-xml-3.1-2007-11": (
        "Musterdateien_GAEB_DA_XML_3.1_20-11-2007.zip",
        {
            "Musterdatei_3.1_2008-001.X81",
            "Musterdatei_3.1_2008-004.X83",
            "Musterdatei_3.1_2008-001.X84",
            "Musterdatei_3.1_2008-004.X86",
        },
    ),
    "gaeb-da-xml-3.2-2013-10": (
        "Zeitvertrag_3.2_2014-03_Beispieldateien.zip",
        {
            "Beispiel_01_83Z_3.2_2014-03-24.X83Z",
            "Beispiel_01_84Z_3.2_2014-03-24.X84Z",
            "Beispiel_01_86ZE_3.2_2014-03-24.X86ZE",
            "Beispiel_01_86ZR_3.2_2014-03-24.X86ZR",
        },
    ),
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def download_archives(manifest: dict, directory: Path) -> None:
    for name, expected in manifest["source_archives"].items():
        request = Request(expected["url"], headers={"User-Agent": "openbimrs-gaeb-reference-fetcher/0.1"})
        with urlopen(request, timeout=90) as response:
            data = response.read()
        actual = sha256(data)
        if actual != expected["sha256"]:
            raise RuntimeError(f"SHA-256 mismatch for {name}: {actual}")
        if len(data) != expected["size"]:
            raise RuntimeError(f"size mismatch for {name}: {len(data)}")
        (directory / name).write_bytes(data)
        with ZipFile(directory / name) as archive:
            broken = archive.testzip()
            if broken:
                raise RuntimeError(f"CRC failure in {name}: {broken}")


def member_name(path: str) -> str:
    member = PurePosixPath(path)
    if member.is_absolute() or ".." in member.parts:
        raise RuntimeError(f"unsafe ZIP member: {path}")
    return member.name


def write_unique(path: Path, data: bytes) -> None:
    if path.exists() and path.read_bytes() != data:
        raise RuntimeError(f"conflicting official payloads for {path.name}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def extract_schema(downloads: Path, stage: Path) -> None:
    for release, archive_names in SCHEMA_RELEASES.items():
        destination = stage / "schema" / release
        for archive_name in archive_names:
            with ZipFile(downloads / archive_name) as archive:
                for member in archive.infolist():
                    if member.is_dir() or not member.filename.lower().endswith(".xsd"):
                        continue
                    write_unique(destination / member_name(member.filename), archive.read(member))


def extract_examples(downloads: Path, stage: Path) -> None:
    for release, (archive_name, selected) in EXAMPLES.items():
        destination = stage / "examples" / release
        found = set()
        with ZipFile(downloads / archive_name) as archive:
            for member in archive.infolist():
                name = member_name(member.filename)
                if member.is_dir() or name not in selected:
                    continue
                write_unique(destination / name, archive.read(member))
                found.add(name)
        missing = selected - found
        if missing:
            raise RuntimeError(f"missing examples in {archive_name}: {sorted(missing)}")


def verify_outputs(manifest: dict, stage: Path) -> None:
    actual_paths = {
        path.relative_to(stage).as_posix()
        for family in ("schema", "examples")
        for path in (stage / family).rglob("*")
        if path.is_file()
    }
    expected_paths = set(manifest["files"])
    if actual_paths != expected_paths:
        raise RuntimeError(
            f"output set differs: missing={sorted(expected_paths - actual_paths)}, "
            f"extra={sorted(actual_paths - expected_paths)}"
        )
    for relative, expected_sha256 in manifest["files"].items():
        path = stage / relative
        data = path.read_bytes()
        if sha256(data) != expected_sha256:
            raise RuntimeError(f"output verification failed: {relative}")


def main() -> None:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory(prefix="gaeb-fetch-") as temporary:
        temporary = Path(temporary)
        downloads = temporary / "downloads"
        stage = temporary / "stage"
        downloads.mkdir()
        download_archives(manifest, downloads)
        extract_schema(downloads, stage)
        extract_examples(downloads, stage)
        verify_outputs(manifest, stage)

        for family in ("schema", "examples"):
            destination = REFERENCES / family
            if destination.exists():
                shutil.rmtree(destination)
            shutil.copytree(stage / family, destination)

    print(f"verified {len(manifest['source_archives'])} archives")
    print(f"installed {len(manifest['files'])} files under {REFERENCES}")


if __name__ == "__main__":
    main()
