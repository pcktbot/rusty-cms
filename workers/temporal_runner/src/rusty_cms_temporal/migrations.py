from __future__ import annotations

import json
import re
import ssl
from dataclasses import asdict, dataclass
from html.parser import HTMLParser
from typing import Any, Iterable
from urllib.parse import urljoin, urlparse
from urllib.request import Request, urlopen

from rusty_cms_temporal.env import env_flag, optional_env


USER_AGENT = "rusty-cms-migrator/0.1"
DISCOVER_SITE_ACTION = "discover_site"
EXTRACT_PAGE_DOCUMENTS_ACTION = "extract_page_documents"
INTERESTING_LAYOUT_TAGS = {"header", "main", "section", "article", "aside", "nav", "footer", "form"}
LAYOUT_KEYWORDS = {
    "hero": "hero",
    "feature": "feature",
    "row": "row",
    "column": "column",
    "col-": "column",
    "grid": "grid",
    "aside": "aside",
    "sidebar": "aside",
    "footer": "footer",
    "header": "header",
    "nav": "nav",
    "gallery": "media",
    "amenit": "grid",
    "card": "grid",
}
MAIN_REGION_TAGS = {"section", "article", "aside", "div", "form"}
WRAPPER_KEYWORDS = {
    "drop-target",
    "before-main",
    "after-main",
    "breadcrumb",
    "masthead",
    "page-shell",
    "content-shell",
}


@dataclass(slots=True)
class FetchResult:
    source_url: str
    final_url: str
    http_status: int | None
    content_type: str | None
    html: str


@dataclass(slots=True)
class CrawledPageArtifact:
    path: str
    source_url: str
    final_url: str
    http_status: int | None
    content_type: str | None
    title_guess: str
    widget_matches: list[str]
    unknown_regions: int
    confidence: float
    warnings: list[str]
    extraction_notes: list[str]
    seo: dict[str, Any]
    schema_types: list[str]
    layout: dict[str, Any]
    text_blocks: list[str]
    images: list[dict[str, Any]]
    media_text_regions: list[dict[str, Any]]
    html_excerpt: str
    document_candidate: dict[str, Any]
    internal_links: list[str]
    asset_urls: list[str]


class PageParser(HTMLParser):
    def __init__(self, source_url: str) -> None:
        super().__init__(convert_charrefs=True)
        self.source_url = source_url
        self._inside_title = False
        self._inside_h1 = False
        self._inside_ld_json = False
        self._inside_text_tag: str | None = None
        self._current_text_parts: list[str] = []
        self._current_anchor_href: str | None = None
        self._current_anchor_text: list[str] = []
        self._current_ld_json_parts: list[str] = []
        self.title_parts: list[str] = []
        self.h1_parts: list[str] = []
        self.internal_links: list[tuple[str, str]] = []
        self.asset_urls: list[str] = []
        self.widget_signals: set[str] = set()
        self.interactive_markers = 0
        self.meta_description: str | None = None
        self.robots: str | None = None
        self.canonical_url: str | None = None
        self.open_graph: dict[str, str] = {}
        self.twitter: dict[str, str] = {}
        self.schema_graph_raw: list[Any] = []
        self.text_blocks: list[str] = []
        self.layout_regions: list[dict[str, Any]] = []
        self.images: list[dict[str, Any]] = []
        self.media_text_regions: list[dict[str, Any]] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attrs_map = {key: value or "" for key, value in attrs}
        class_names = attrs_map.get("class", "")
        element_id = attrs_map.get("id", "")

        if tag == "title":
            self._inside_title = True
        elif tag == "h1":
            self._inside_h1 = True
        elif tag == "a":
            self._current_anchor_href = attrs_map.get("href")
            self._current_anchor_text = []
        elif tag == "script":
            script_type = attrs_map.get("type", "").strip().lower()
            src = attrs_map.get("src", "").strip()
            if src:
                self.asset_urls.append(src)
                self._classify_widget_signal(src)
            if script_type == "application/ld+json":
                self._inside_ld_json = True
                self._current_ld_json_parts = []
            self.interactive_markers += 1
        elif tag in {"form", "video", "iframe"}:
            self.interactive_markers += 1
        elif tag == "img":
            image = self._build_image_candidate(tag, attrs_map, class_names, element_id)
            if image:
                self.images.append(image)

        if tag in {"p", "li", "h2", "h3"}:
            self._inside_text_tag = tag
            self._current_text_parts = []

        if tag == "meta":
            self._collect_meta(attrs_map)
        elif tag == "link" and attrs_map.get("rel", "").lower() == "canonical":
            href = attrs_map.get("href", "").strip()
            if href:
                self.canonical_url = href

        data_widget = attrs_map.get("data-widget", "").strip()
        if data_widget:
            self.widget_signals.add(data_widget)

        self._classify_widget_signal(class_names)
        self._classify_widget_signal(element_id)
        self._collect_layout_region(tag, attrs_map, class_names, element_id)

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self._inside_title = False
        elif tag == "h1":
            self._inside_h1 = False
        elif tag == "a":
            if self._current_anchor_href:
                normalized = normalize_internal_link(self.source_url, self._current_anchor_href)
                if normalized:
                    anchor_text = normalize_text(" ".join(self._current_anchor_text))
                    self.internal_links.append((normalized, anchor_text))
            self._current_anchor_href = None
            self._current_anchor_text = []
        elif tag == "script" and self._inside_ld_json:
            raw = "".join(self._current_ld_json_parts).strip()
            if raw:
                try:
                    self.schema_graph_raw.append(json.loads(raw))
                except json.JSONDecodeError:
                    self.schema_graph_raw.append({"parse_error": True, "raw": raw[:1000]})
            self._inside_ld_json = False
            self._current_ld_json_parts = []

        if tag == self._inside_text_tag:
            text = normalize_text(" ".join(self._current_text_parts))
            if text and text not in self.text_blocks:
                self.text_blocks.append(text)
            self._inside_text_tag = None
            self._current_text_parts = []

    def handle_data(self, data: str) -> None:
        if self._inside_title:
            self.title_parts.append(data)
        if self._inside_h1:
            self.h1_parts.append(data)
        if self._current_anchor_href is not None:
            self._current_anchor_text.append(data)
        if self._inside_ld_json:
            self._current_ld_json_parts.append(data)
        if self._inside_text_tag is not None:
            self._current_text_parts.append(data)

    def _collect_meta(self, attrs_map: dict[str, str]) -> None:
        name = attrs_map.get("name", "").strip().lower()
        property_name = attrs_map.get("property", "").strip().lower()
        content = attrs_map.get("content", "").strip()
        if not content:
            return
        if name == "description":
            self.meta_description = content
        elif name == "robots":
            self.robots = content
        elif name.startswith("twitter:"):
            self.twitter[name] = content
        elif property_name.startswith("og:"):
            self.open_graph[property_name] = content

    def _classify_widget_signal(self, value: str) -> None:
        lowered = value.lower()
        if "floorplan" in lowered or "floor-plan" in lowered:
            self.widget_signals.add("floor-plans-plus")
        if "hero" in lowered and "banner" in lowered:
            self.widget_signals.add("hero-banner")
        if "widget" in lowered and "rich-text" in lowered:
            self.widget_signals.add("rich-text")

    def _collect_layout_region(
        self,
        tag: str,
        attrs_map: dict[str, str],
        class_names: str,
        element_id: str,
    ) -> None:
        kind = None
        lowered = f"{tag} {class_names} {element_id}".lower()
        if tag in INTERESTING_LAYOUT_TAGS:
            kind = tag
        else:
            for keyword, mapped_kind in LAYOUT_KEYWORDS.items():
                if keyword in lowered:
                    kind = mapped_kind
                    break

        if not kind:
            return

        selector_hint = build_selector_hint(tag, class_names, element_id)
        region = {
            "kind": kind,
            "selector_hint": selector_hint,
        }
        if region not in self.layout_regions:
            self.layout_regions.append(region)

    def _build_image_candidate(
        self,
        tag: str,
        attrs_map: dict[str, str],
        class_names: str,
        element_id: str,
    ) -> dict[str, Any] | None:
        src = attrs_map.get("src", "").strip()
        if not src:
            return None

        role_hint = infer_role_hint(f"{class_names} {element_id}")
        image = {
            "src": src,
            "srcset": attrs_map.get("srcset", "").strip() or None,
            "sizes": attrs_map.get("sizes", "").strip() or None,
            "alt": attrs_map.get("alt", "").strip() or None,
            "title": attrs_map.get("title", "").strip() or None,
            "width": parse_int(attrs_map.get("width")),
            "height": parse_int(attrs_map.get("height")),
            "selector_hint": build_selector_hint(tag, class_names, element_id),
            "role_hint": role_hint,
        }
        return image


class MainContentParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self._inside_main = False
        self._stack: list[dict[str, Any]] = []
        self._regions: list[dict[str, Any]] = []
        self._region_counter = 0

    @property
    def regions(self) -> list[dict[str, Any]]:
        return self._regions

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attrs_map = {key: value or "" for key, value in attrs}
        class_names = attrs_map.get("class", "")
        element_id = attrs_map.get("id", "")

        if tag == "main" and not self._inside_main:
            self._inside_main = True
            self._stack.append({"tag": tag, "wrapper": False, "region_index": None})
            return

        if not self._inside_main:
            return

        parent_region_index = self._current_region_index()
        wrapper = is_wrapper_container(tag, class_names, element_id)
        region_index = parent_region_index

        if (
            region_index is None
            and not wrapper
            and tag in MAIN_REGION_TAGS
        ):
            region_index = self._start_region(tag, class_names, element_id)

        if tag == "img" and region_index is not None:
            image = {
                "src": attrs_map.get("src", "").strip(),
                "alt": attrs_map.get("alt", "").strip() or None,
                "title": attrs_map.get("title", "").strip() or None,
                "selector_hint": build_selector_hint(tag, class_names, element_id),
                "role_hint": infer_role_hint(f"{class_names} {element_id}"),
            }
            if image["src"]:
                self._regions[region_index]["images"].append(image)

        self._stack.append(
            {
                "tag": tag,
                "wrapper": wrapper,
                "region_index": region_index,
            }
        )

    def handle_endtag(self, tag: str) -> None:
        if not self._inside_main:
            return

        if self._stack:
            self._stack.pop()

        if tag == "main":
            self._inside_main = False

    def handle_data(self, data: str) -> None:
        if not self._inside_main:
            return

        region_index = self._current_region_index()
        if region_index is None:
            return

        text = normalize_text(data)
        if not text:
            return

        region = self._regions[region_index]
        region["text_parts"].append(text)
        if region["heading"] is None and len(text) <= 120:
            region["heading"] = text

    def _start_region(self, tag: str, class_names: str, element_id: str) -> int:
        selector_hint = build_selector_hint(tag, class_names, element_id)
        kind = classify_main_region(tag, class_names, element_id)
        region = {
            "id": f"main_region_{self._region_counter + 1}",
            "kind": kind,
            "selector_hint": selector_hint,
            "images": [],
            "text_parts": [],
            "heading": None,
        }
        self._region_counter += 1
        self._regions.append(region)
        return len(self._regions) - 1

    def _current_region_index(self) -> int | None:
        for node in reversed(self._stack):
            if node["region_index"] is not None:
                return int(node["region_index"])
        return None

def normalize_text(value: str) -> str:
    return " ".join(value.split()).strip()


def normalize_internal_link(source_url: str, href: str) -> str | None:
    if not href or href.startswith("#") or href.startswith("mailto:") or href.startswith("tel:"):
        return None

    joined = urljoin(source_url, href)
    parsed = urlparse(joined)
    source = urlparse(source_url)
    if parsed.scheme not in {"http", "https"}:
        return None
    if parsed.netloc != source.netloc:
        return None
    return parsed.path or "/"


def build_selector_hint(tag: str, class_names: str, element_id: str) -> str:
    class_part = ""
    first_class = next((item for item in class_names.split() if item), None)
    if first_class:
        class_part = f".{first_class}"
    id_part = f"#{element_id}" if element_id else ""
    return f"{tag}{id_part}{class_part}"


def infer_role_hint(value: str) -> str | None:
    lowered = value.lower()
    for keyword, role in {
        "hero": "hero",
        "gallery": "gallery",
        "card": "card",
        "feature": "feature",
        "amenit": "feature",
    }.items():
        if keyword in lowered:
            return role
    return None


def is_wrapper_container(tag: str, class_names: str, element_id: str) -> bool:
    if tag not in MAIN_REGION_TAGS:
        return False

    lowered = f"{class_names} {element_id}".lower()
    return any(keyword in lowered for keyword in WRAPPER_KEYWORDS)


def classify_main_region(tag: str, class_names: str, element_id: str) -> str:
    lowered = f"{tag} {class_names} {element_id}".lower()
    for keyword, mapped_kind in LAYOUT_KEYWORDS.items():
        if keyword in lowered:
            return mapped_kind
    return tag


def parse_int(value: str | None) -> int | None:
    if not value:
        return None
    try:
        return int(value)
    except ValueError:
        return None


def build_ssl_context() -> ssl.SSLContext:
    context = ssl.create_default_context()
    ca_bundle = optional_env("CMS_MIGRATION_CA_BUNDLE")
    if ca_bundle:
        context.load_verify_locations(cafile=ca_bundle)

    if env_flag("CMS_MIGRATION_ALLOW_INSECURE_TLS", default=False):
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE

    return context


def fetch_html(url: str, timeout_seconds: float = 10.0) -> FetchResult:
    request = Request(url, headers={"User-Agent": USER_AGENT})
    with urlopen(request, timeout=timeout_seconds, context=build_ssl_context()) as response:
        charset = response.headers.get_content_charset() or "utf-8"
        html = response.read().decode(charset, errors="replace")
        return FetchResult(
            source_url=url,
            final_url=response.geturl(),
            http_status=getattr(response, "status", None),
            content_type=response.headers.get_content_type(),
            html=html,
        )


def discover_paths(homepage_url: str, html: str) -> list[str]:
    parser = PageParser(homepage_url)
    parser.feed(html)

    homepage_path = urlparse(homepage_url).path or "/"
    paths = [homepage_path]
    seen_paths = {homepage_path}
    for path, _anchor_text in unique_links(parser.internal_links):
        if path in seen_paths:
            continue
        seen_paths.add(path)
        paths.append(path)
        if len(paths) >= 8:
            break
    return paths


def crawl_page(path: str, page_url: str, detect_widgets: bool) -> CrawledPageArtifact:
    warnings: list[str] = []
    extraction_notes: list[str] = []
    fetch = fetch_html(page_url)
    parser = PageParser(fetch.final_url)
    parser.feed(fetch.html)

    title_guess = normalize_text(" ".join(parser.title_parts)) or normalize_text(
        " ".join(parser.h1_parts)
    )
    if not title_guess:
        title_guess = humanize_path(path)

    schema_types = sorted(extract_schema_types(parser.schema_graph_raw))
    layout = {
        "regions": parser.layout_regions[:24],
        "counts": summarize_layout_counts(parser.layout_regions),
    }
    text_blocks = parser.text_blocks[:12]
    images = parser.images[:16]
    media_text_regions = infer_media_text_regions(layout["regions"], images, text_blocks)[:12]
    widget_matches = sorted(parser.widget_signals) if detect_widgets else []
    unknown_regions = max(parser.interactive_markers - len(widget_matches), 0)
    html_excerpt = extract_html_excerpt(fetch.html)
    document_candidate = build_document_candidate(
        title_guess,
        widget_matches,
        layout,
        text_blocks,
        images,
        media_text_regions,
    )

    if schema_types:
        extraction_notes.append(f"discovered schema types: {', '.join(schema_types)}")
    if layout["regions"]:
        extraction_notes.append(
            "identified layout regions: "
            + ", ".join(region["kind"] for region in layout["regions"][:8])
        )
    else:
        extraction_notes.append("no major layout regions identified in first-pass summary")

    return CrawledPageArtifact(
        path=path,
        source_url=page_url,
        final_url=fetch.final_url,
        http_status=fetch.http_status,
        content_type=fetch.content_type,
        title_guess=title_guess,
        widget_matches=widget_matches,
        unknown_regions=unknown_regions,
        confidence=0.85 if fetch.http_status == 200 else 0.4,
        warnings=warnings,
        extraction_notes=extraction_notes,
        seo={
            "title": title_guess,
            "meta_description": parser.meta_description,
            "h1": normalize_text(" ".join(parser.h1_parts)) or None,
            "canonical_url": parser.canonical_url,
            "robots": parser.robots,
            "open_graph": parser.open_graph,
            "twitter": parser.twitter,
            "schema_graph_raw": parser.schema_graph_raw,
        },
        schema_types=schema_types,
        layout=layout,
        text_blocks=text_blocks,
        images=images,
        media_text_regions=media_text_regions,
        html_excerpt=html_excerpt,
        document_candidate=document_candidate,
        internal_links=[path for path, _anchor_text in unique_links(parser.internal_links)[:24]],
        asset_urls=sorted(set(parser.asset_urls))[:32],
    )


def extract_main_regions(html: str) -> list[dict[str, Any]]:
    parser = MainContentParser()
    parser.feed(html)

    regions: list[dict[str, Any]] = []
    for region in parser.regions:
        text_parts = region.get("text_parts", [])
        heading = region.get("heading")
        body_candidates = [part for part in text_parts if part != heading]
        body = " ".join(body_candidates[:3]) if body_candidates else (text_parts[0] if text_parts else "")
        cleaned = {
            "id": region["id"],
            "kind": region["kind"],
            "selector_hint": region["selector_hint"],
            "heading": heading,
            "body": body,
            "images": region.get("images", [])[:4],
        }
        if cleaned["heading"] or cleaned["body"] or cleaned["images"]:
            regions.append(cleaned)
    return regions[:18]


def build_document_candidate_from_main_regions(
    title: str,
    main_regions: list[dict[str, Any]],
    widget_matches: list[str],
) -> dict[str, Any]:
    blocks: list[dict[str, Any]] = []

    for region in main_regions:
        images = region.get("images") or []
        heading = region.get("heading")
        body = region.get("body")
        selector_hint = region.get("selector_hint")
        kind = region.get("kind")

        if images and (heading or body):
            blocks.append(
                {
                    "kind": "primitive",
                    "primitive_type": "media_text",
                    "selector_hint": selector_hint,
                    "content": {
                        "heading": heading,
                        "body": body,
                        "image": images[0],
                    },
                }
            )
            continue

        if images:
            blocks.append(
                {
                    "kind": "primitive",
                    "primitive_type": "image",
                    "selector_hint": selector_hint,
                    "content": images[0],
                }
            )

        if heading or body:
            blocks.append(
                {
                    "kind": "primitive",
                    "primitive_type": "rich_text",
                    "selector_hint": selector_hint,
                    "content": {
                        "heading": heading,
                        "text": body or heading,
                        "region_kind": kind,
                    },
                }
            )

    for widget_slug in widget_matches:
        blocks.append(
            {
                "kind": "widget",
                "widget_slug": widget_slug,
                "settings": {"migration_detected": True},
            }
        )

    return {
        "title": title,
        "regions": {
            "main": blocks[:18],
        },
    }


def extract_page_document_candidate(
    path: str,
    page_url: str,
    title_guess: str,
    widget_matches: list[str],
) -> CrawledPageArtifact:
    warnings: list[str] = []
    extraction_notes: list[str] = []
    fetch = fetch_html(page_url)
    parser = PageParser(fetch.final_url)
    parser.feed(fetch.html)

    title_guess = normalize_text(" ".join(parser.title_parts)) or normalize_text(
        " ".join(parser.h1_parts)
    ) or title_guess
    schema_types = sorted(extract_schema_types(parser.schema_graph_raw))
    layout = {
        "regions": parser.layout_regions[:24],
        "counts": summarize_layout_counts(parser.layout_regions),
    }
    text_blocks = parser.text_blocks[:12]
    images = parser.images[:16]
    media_text_regions = infer_media_text_regions(layout["regions"], images, text_blocks)[:12]
    html_excerpt = extract_html_excerpt(fetch.html)
    main_regions = extract_main_regions(fetch.html)
    document_candidate = build_document_candidate_from_main_regions(
        title_guess,
        main_regions,
        sorted(parser.widget_signals),
    )

    extraction_notes.append(
        f"page-document extraction walked {len(main_regions)} ordered main-content regions"
    )
    if main_regions:
        extraction_notes.append(
            "main regions: " + ", ".join(region["kind"] for region in main_regions[:8])
        )
    else:
        extraction_notes.append("no ordered main-content regions found; falling back to discovery summary")

    return CrawledPageArtifact(
        path=path,
        source_url=page_url,
        final_url=fetch.final_url,
        http_status=fetch.http_status,
        content_type=fetch.content_type,
        title_guess=title_guess,
        widget_matches=sorted(parser.widget_signals),
        unknown_regions=max(parser.interactive_markers - len(parser.widget_signals), 0),
        confidence=0.92 if main_regions else 0.55,
        warnings=warnings,
        extraction_notes=extraction_notes,
        seo={
            "title": title_guess,
            "meta_description": parser.meta_description,
            "h1": normalize_text(" ".join(parser.h1_parts)) or None,
            "canonical_url": parser.canonical_url,
            "robots": parser.robots,
            "open_graph": parser.open_graph,
            "twitter": parser.twitter,
            "schema_graph_raw": parser.schema_graph_raw,
        },
        schema_types=schema_types,
        layout={
            "regions": main_regions[:24],
            "counts": summarize_layout_counts(main_regions),
        },
        text_blocks=text_blocks,
        images=images,
        media_text_regions=media_text_regions,
        html_excerpt=html_excerpt,
        document_candidate=document_candidate,
        internal_links=[path for path, _anchor_text in unique_links(parser.internal_links)[:24]],
        asset_urls=sorted(set(parser.asset_urls))[:32],
    )


def build_document_candidate(
    title: str,
    widget_matches: list[str],
    layout: dict[str, Any],
    text_blocks: list[str],
    images: list[dict[str, Any]],
    media_text_regions: list[dict[str, Any]],
) -> dict[str, Any]:
    blocks: list[dict[str, Any]] = []
    for region in media_text_regions[:4]:
        blocks.append(
            {
                "kind": "media_text",
                "selector_hint": region.get("selector_hint"),
                "orientation": region.get("orientation"),
                "image": region.get("image"),
                "text": region.get("text"),
            }
        )

    for widget_slug in widget_matches:
        blocks.append(
            {
                "kind": "widget",
                "widget_slug": widget_slug,
                "settings": {"migration_detected": True},
            }
        )

    for image in images[:6]:
        blocks.append(
            {
                "kind": "image",
                "src": image.get("src"),
                "alt": image.get("alt"),
                "role": image.get("role_hint"),
            }
        )

    for region in layout.get("regions", [])[:6]:
        blocks.append(
            {
                "kind": "layout_region",
                "region_kind": region.get("kind"),
                "selector_hint": region.get("selector_hint"),
            }
        )

    for text in text_blocks[:6]:
        blocks.append(
            {
                "kind": "text_excerpt",
                "content": text,
            }
        )

    return {
        "title": title,
        "regions": {
            "main": blocks,
        },
    }


def infer_media_text_regions(
    layout_regions: list[dict[str, Any]],
    images: list[dict[str, Any]],
    text_blocks: list[str],
) -> list[dict[str, Any]]:
    if not images or not text_blocks:
        return []

    orientation = "image_left"
    selector_hint = next(
        (
            region.get("selector_hint")
            for region in layout_regions
            if region.get("kind") in {"section", "article", "hero", "row"}
        ),
        "section.media-text",
    )
    all_selectors = " ".join(
        str(region.get("selector_hint") or "") for region in layout_regions
    ).lower()
    if "right" in all_selectors:
        orientation = "image_right"

    heading = text_blocks[0]
    body = " ".join(text_blocks[1:3]) if len(text_blocks) > 1 else text_blocks[0]

    return [
        {
            "kind": "media_text",
            "selector_hint": selector_hint,
            "orientation": orientation,
            "image": images[0],
            "text": {
                "heading": heading,
                "body": body,
            },
        }
    ]


def extract_html_excerpt(html: str) -> str:
    match = re.search(r"<main\b[^>]*>(.*?)</main>", html, flags=re.IGNORECASE | re.DOTALL)
    if not match:
        match = re.search(r"<body\b[^>]*>(.*?)</body>", html, flags=re.IGNORECASE | re.DOTALL)
    excerpt = match.group(1) if match else html
    excerpt = excerpt.strip()
    if len(excerpt) > 4000:
        excerpt = excerpt[:4000]
    return excerpt


def summarize_layout_counts(regions: list[dict[str, Any]]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for region in regions:
        kind = str(region.get("kind") or "unknown")
        counts[kind] = counts.get(kind, 0) + 1
    return counts


def extract_schema_types(items: list[Any]) -> set[str]:
    types: set[str] = set()

    def visit(node: Any) -> None:
        if isinstance(node, dict):
            type_value = node.get("@type")
            if isinstance(type_value, str):
                types.add(type_value)
            elif isinstance(type_value, list):
                for item in type_value:
                    if isinstance(item, str):
                        types.add(item)
            for value in node.values():
                visit(value)
        elif isinstance(node, list):
            for item in node:
                visit(item)

    for item in items:
        visit(item)
    return types


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
    action = str(payload.get("action") or DISCOVER_SITE_ACTION)
    options = dict(payload.get("options") or {})
    homepage_url = str(payload.get("homepage_url") or "")
    if action == EXTRACT_PAGE_DOCUMENTS_ACTION:
        return await execute_page_document_extraction(request)
    detect_widgets = bool(options.get("detect_registered_widgets", False))
    warnings: list[str] = []

    try:
        homepage_fetch = fetch_html(homepage_url)
        paths = discover_paths(homepage_url, homepage_fetch.html)
        pages: list[CrawledPageArtifact] = []
        for path in paths:
            page_url = urljoin(homepage_url, path)
            try:
                pages.append(crawl_page(path, page_url, detect_widgets))
            except Exception as error:
                warnings.append(f"crawl failed for {path}: {error}")
                pages.append(
                    CrawledPageArtifact(
                        path=path,
                        source_url=page_url,
                        final_url=page_url,
                        http_status=None,
                        content_type=None,
                        title_guess=humanize_path(path),
                        widget_matches=[],
                        unknown_regions=1,
                        confidence=0.2,
                        warnings=[f"page crawl failed: {error}"],
                        extraction_notes=["fallback placeholder generated for failed crawl"],
                        seo={
                            "title": humanize_path(path),
                            "meta_description": None,
                            "h1": None,
                            "canonical_url": None,
                            "robots": None,
                            "open_graph": {},
                            "twitter": {},
                            "schema_graph_raw": [],
                        },
                        schema_types=[],
                        layout={"regions": [], "counts": {}},
                        text_blocks=[],
                        images=[],
                        media_text_regions=[],
                        html_excerpt="",
                        document_candidate={"title": humanize_path(path), "regions": {"main": []}},
                        internal_links=[],
                        asset_urls=[],
                    )
                )
    except Exception as error:
        warnings.append(f"homepage crawl failed: {error}")
        pages = [
            CrawledPageArtifact(
                path=urlparse(homepage_url).path or "/",
                source_url=homepage_url,
                final_url=homepage_url,
                http_status=None,
                content_type=None,
                title_guess="Homepage",
                widget_matches=["registry-detection-pending"] if detect_widgets else [],
                unknown_regions=1,
                confidence=0.2,
                warnings=[f"homepage crawl failed: {error}"],
                extraction_notes=["fallback placeholder generated because homepage crawl failed"],
                seo={
                    "title": "Homepage",
                    "meta_description": None,
                    "h1": None,
                    "canonical_url": None,
                    "robots": None,
                    "open_graph": {},
                    "twitter": {},
                    "schema_graph_raw": [],
                },
                schema_types=[],
                layout={"regions": [], "counts": {}},
                text_blocks=[],
                images=[],
                media_text_regions=[],
                html_excerpt="",
                document_candidate={"title": "Homepage", "regions": {"main": []}},
                internal_links=[],
                asset_urls=[],
            )
        ]

    warnings.append(
        "Discovery now captures SEO, schema hints, layout summaries, and provisional document candidates, but widget reconstruction and full DOM import are still pending."
    )
    if optional_env("CMS_MIGRATION_CA_BUNDLE"):
        warnings.append("Migration crawler is using a custom CA bundle.")
    if env_flag("CMS_MIGRATION_ALLOW_INSECURE_TLS", default=False):
        warnings.append(
            "Migration crawler is running with insecure TLS verification disabled."
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
            "pages": [asdict(page) for page in pages],
            "warnings": warnings,
        },
    }


async def execute_page_document_extraction(request: dict) -> dict:
    payload = dict(request.get("input_payload") or {})
    pages = list(payload.get("pages") or [])
    warnings: list[str] = []
    extracted_pages: list[dict[str, Any]] = []

    for page in pages:
        path = str(page.get("path") or "/")
        page_url = str(page.get("source_url") or page.get("final_url") or "")
        title_guess = str(page.get("title_guess") or humanize_path(path))
        if not page_url:
            warnings.append(f"page extraction skipped for {path}: source_url missing")
            continue
        try:
            artifact = extract_page_document_candidate(
                path=path,
                page_url=page_url,
                title_guess=title_guess,
                widget_matches=list(page.get("widget_matches") or []),
            )
            extracted_pages.append(asdict(artifact))
        except Exception as error:
            warnings.append(f"page extraction failed for {path}: {error}")

    warnings.append(
        "Page-document extraction uses ordered main-content walking and wrapper suppression, but widget reconstruction is still provisional."
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
            "action": EXTRACT_PAGE_DOCUMENTS_ACTION,
            "migration_job_id": payload.get("migration_job_id"),
            "homepage_url": payload.get("homepage_url"),
            "client_id": payload.get("client_id"),
            "location_id": payload.get("location_id"),
            "page_count_guess": len(extracted_pages),
            "pages": extracted_pages,
            "warnings": warnings,
        },
    }
