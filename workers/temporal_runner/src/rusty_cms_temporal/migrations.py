from __future__ import annotations

from urllib.parse import urlparse


def _homepage_path(homepage_url: str) -> str:
    parsed = urlparse(homepage_url)
    return parsed.path or "/"


async def execute_site_migration(request: dict) -> dict:
    payload = dict(request.get("input_payload") or {})
    options = dict(payload.get("options") or {})
    homepage_url = str(payload.get("homepage_url") or "")
    page_path = _homepage_path(homepage_url)

    widget_matches = []
    if options.get("detect_registered_widgets", False):
        widget_matches.append("registry-detection-pending")

    warnings = [
        "Migration workflow foundation is scaffolded; crawler, classifier, and importer are not implemented yet.",
    ]
    if options.get("use_legacy_api_enrichment", False):
        warnings.append(
            "Legacy API enrichment is enabled in the request contract but not implemented yet."
        )

    return {
        "accepted": True,
        "workflow_kind": request["kind"],
        "site_id": request["site_id"],
        "branch_name": request["branch_name"],
        "requested_runtime": request["requested_runtime"],
        "temporal_queue": request["temporal_queue"],
        "migration": {
            "status": "review_ready",
            "homepage_url": homepage_url,
            "client_id": payload.get("client_id"),
            "location_id": payload.get("location_id"),
            "page_count_guess": 1,
            "pages": [
                {
                    "path": page_path,
                    "title_guess": "Homepage",
                    "widget_matches": widget_matches,
                    "unknown_regions": 1,
                    "confidence": 0.25,
                }
            ],
            "warnings": warnings,
        },
    }
