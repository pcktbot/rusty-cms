from __future__ import annotations

from dataclasses import dataclass
from html.parser import HTMLParser
from typing import Iterable
from urllib.parse import urljoin, urlparse
from urllib.request import Request, urlopen


USER_AGENT = "rusty-cms-migrator/0.1"


@dataclass(slots=True)
class DiscoveredPage:
    path: str
    title_guess: str
    widget_matches: list[str]
    unknown_regions: int
    confidence: float


class HomepageParser(HTMLParser):
    def __init__(self, homepage_url: str) -> None:
        super().__init__(convert_charrefs=True)
        self.homepage_url = homepage_url
        self.base_host = urlparse(homepage_url).netloc
        self.base_path = urlparse(homepage_url).path or "/"
        self._inside_title = False
        self._inside_h1 = False
        self._current_anchor_href: str | None = None
        self._current_anchor_text: list[str] = []
        self.title_parts: list[str] = []
        self.h1_parts: list[str] = []
        self.internal_links: list[tuple[str, str]] = []
        self.script_paths: list[str] = []
        self.widget_signals: set[str] = set()
        self.interactive_markers = 0
        self.meta_description: str | None = None

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attrs_map = {key: value or "" for key, value in attrs}
        if tag == "title":
            self._inside_title = True
        elif tag == "h1":
            self._inside_h1 = True
        elif tag == "a":
            self._current_anchor_href = attrs_map.get("href")
            self._current_anchor_text = []
        elif tag == "script":
            src = attrs_map.get("src", "").strip()
            if src:
                self.script_paths.append(src)
                self._classify_widget_signal(src)
            self.interactive_markers += 1
        elif tag in {"form", "video", "iframe"}:
            self.interactive_markers += 1
        elif tag == "meta" and attrs_map.get("name", "").lower() == "description":
            content = attrs_map.get("content", "").strip()
            if content:
                self.meta_description = content

        data_widget = attrs_map.get("data-widget", "").strip()
        if data_widget:
            self.widget_signals.add(data_widget)
        element_id = attrs_map.get("id", "")
        class_names = attrs_map.get("class", "")
        self._classify_widget_signal(element_id)
        self._classify_widget_signal(class_names)

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self._inside_title = False
        elif tag == "h1":
            self._inside_h1 = False
        elif tag == "a":
            if self._current_anchor_href:
                normalized = normalize_internal_link(
                    self.homepage_url, self._current_anchor_href
                )
                if normalized:
                    anchor_text = normalize_text(" ".join(self._current_anchor_text))
                    self.internal_links.append((normalized, anchor_text))
            self._current_anchor_href = None
            self._current_anchor_text = []

    def handle_data(self, data: str) -> None:
        if self._inside_title:
            self.title_parts.append(data)
        if self._inside_h1:
            self.h1_parts.append(data)
        if self._current_anchor_href is not None:
            self._current_anchor_text.append(data)

    def _classify_widget_signal(self, value: str) -> None:
        lowered = value.lower()
        if "floorplan" in lowered or "floor-plan" in lowered:
            self.widget_signals.add("floor-plans-plus")
        if "hero" in lowered and "banner" in lowered:
            self.widget_signals.add("hero-banner")
        if "widget" in lowered and "rich-text" in lowered:
            self.widget_signals.add("rich-text")


def normalize_text(value: str) -> str:
    return " ".join(value.split()).strip()


def normalize_internal_link(homepage_url: str, href: str) -> str | None:
    if not href or href.startswith("#") or href.startswith("mailto:") or href.startswith("tel:"):
        return None

    joined = urljoin(homepage_url, href)
    parsed = urlparse(joined)
    homepage = urlparse(homepage_url)
    if parsed.scheme not in {"http", "https"}:
        return None
    if parsed.netloc != homepage.netloc:
        return None
    return parsed.path or "/"


def fetch_html(url: str, timeout_seconds: float = 10.0) -> str:
    request = Request(url, headers={"User-Agent": USER_AGENT})
    with urlopen(request, timeout=timeout_seconds) as response:
        charset = response.headers.get_content_charset() or "utf-8"
        return response.read().decode(charset, errors="replace")


def discover_pages(homepage_url: str, html: str, detect_widgets: bool) -> list[DiscoveredPage]:
    parser = HomepageParser(homepage_url)
    parser.feed(html)

    homepage_title = normalize_text(" ".join(parser.title_parts)) or normalize_text(
        " ".join(parser.h1_parts)
    )
    if not homepage_title:
        homepage_title = "Homepage"

    homepage_page = DiscoveredPage(
        path=urlparse(homepage_url).path or "/",
        title_guess=homepage_title,
        widget_matches=sorted(parser.widget_signals) if detect_widgets else [],
        unknown_regions=max(parser.interactive_markers - len(parser.widget_signals), 0),
        confidence=0.7 if homepage_title else 0.35,
    )

    pages: list[DiscoveredPage] = [homepage_page]
    seen_paths = {homepage_page.path}

    for path, anchor_text in unique_links(parser.internal_links):
        if path in seen_paths:
            continue
        seen_paths.add(path)
        pages.append(
            DiscoveredPage(
                path=path,
                title_guess=anchor_text or humanize_path(path),
                widget_matches=[],
                unknown_regions=0,
                confidence=0.4 if anchor_text else 0.25,
            )
        )
        if len(pages) >= 8:
            break

    return pages


def unique_links(links: Iterable[tuple[str, str]]) -> list[tuple[str, str]]:
    unique: list[tuple[str, str]] = []
    seen: set[str] = set()
    for path, anchor_text in links:
        if path in seen:
            continue
        seen.add(path)
        unique.append((path, anchor_text))
    return unique


def humanize_path(path: str) -> str:
    stripped = path.strip("/")
    if not stripped:
        return "Homepage"
    return stripped.replace("-", " ").replace("_", " ").title()


async def execute_site_migration(request: dict) -> dict:
    payload = dict(request.get("input_payload") or {})
    options = dict(payload.get("options") or {})
    homepage_url = str(payload.get("homepage_url") or "")
    detect_widgets = bool(options.get("detect_registered_widgets", False))
    warnings: list[str] = []

    try:
        html = fetch_html(homepage_url)
        pages = discover_pages(homepage_url, html, detect_widgets)
    except Exception as error:
        warnings.append(f"homepage crawl failed: {error}")
        pages = [
            DiscoveredPage(
                path=urlparse(homepage_url).path or "/",
                title_guess="Homepage",
                widget_matches=["registry-detection-pending"] if detect_widgets else [],
                unknown_regions=1,
                confidence=0.2,
            )
        ]

    warnings.append(
        "Discovery is implemented for the homepage and same-host links, but classifier, importer, and validation are still pending."
    )
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
            "page_count_guess": len(pages),
            "pages": [
                {
                    "path": page.path,
                    "title_guess": page.title_guess,
                    "widget_matches": page.widget_matches,
                    "unknown_regions": page.unknown_regions,
                    "confidence": page.confidence,
                }
                for page in pages
            ],
            "warnings": warnings,
        },
    }
