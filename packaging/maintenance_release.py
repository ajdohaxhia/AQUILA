# Copyright (C) 2026 Michael Lane
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

"""Recognize an explicitly reserved release without a changelog entry."""
from pathlib import Path


def is_maintenance_build(root: Path, version: int) -> bool:
    marker = root / "packaging/maintenance-build.txt"
    return marker.is_file() and marker.read_text(encoding="utf-8").strip() == str(version)
