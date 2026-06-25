from __future__ import annotations

import importlib


BUILDERS = [
    "build_user_guide",
    "build_reference_bundle",
    "build_release_notes",
]


def main() -> None:
    for name in BUILDERS:
        importlib.import_module(name).main()


if __name__ == "__main__":
    main()
