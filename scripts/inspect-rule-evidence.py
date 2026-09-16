"""Read-only evidence discovery. Output is NOT a rule-conformance test.

Only same-origin public JavaScript is inspected. Do not run downloaded code,
submit games, authenticate, or treat absent matches as verified rules.
"""
import hashlib
import json
import re
import urllib.parse
import urllib.request

BASE = "https://tetr.io/"
LIMIT = 24 * 1024 * 1024
HEADERS = {"User-Agent": "Mozilla/5.0"}
KEYS = ["allclear_garbage", "allclear", "b2bcharging", "garbagespecial", "all-mini", "openerphase", "garbagebonus", "clutch", "garbage_multiplier"]


def read(url):
    if urllib.parse.urlsplit(url).netloc != "tetr.io":
        raise ValueError("only tetr.io public client resources are allowed")
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=25) as response:
        raw = response.read(LIMIT + 1)
    if len(raw) > LIMIT:
        raise ValueError("public resource exceeds inspection size limit")
    return raw.decode("utf-8")


def main():
    report = {"parity_verified": False, "resources": [], "errors": []}
    html = read(BASE)
    refs = re.findall(r'<script[^>]+src=[\"\x27]([^\"\x27]+)', html)
    pending = [urllib.parse.urljoin(BASE, r) for r in refs if "bootstrap" in r or "tetrio" in r]
    seen = set()
    print("PUBLIC_SCRIPT_REFERENCES", json.dumps(refs))
    while pending and len(seen) < 5:
        url = pending.pop(0)
        if url in seen or urllib.parse.urlsplit(url).netloc != "tetr.io":
            continue
        seen.add(url)
        try:
            text = read(url)
            record = {"url": url, "bytes": len(text.encode()), "sha256": hashlib.sha256(text.encode()).hexdigest(), "key_matches": {}}
            print("RESOURCE", json.dumps(record))
            if "bootstrap" in url:
                print("BOOTSTRAP_PREFIX", text[:1200])
            for key in KEYS:
                hits = list(re.finditer(re.escape(key), text, re.I))
                record["key_matches"][key] = len(hits)
                for match in hits[:3]:
                    print("KEY", key, text[max(0, match.start()-160):match.end()+400])
            # Literal JS references only; no evaluation or de-obfuscation.
            found = re.findall(r'[\"\x27]([^\"\x27\s<>]+\.js(?:\?[^\"\x27\s<>]*)?)[\"\x27]', text)
            found = [urllib.parse.urljoin(url, ref) for ref in found if "tetrio" in ref or "/js/" in ref]
            print("DISCOVERED_JS", json.dumps(found[:20]))
            pending.extend(ref for ref in found if ref not in seen)
            report["resources"].append(record)
        except Exception as exc:
            report["errors"].append({"url": url, "error": str(exc)})
    print("EVIDENCE_REPORT", json.dumps(report))
    with open("rule-evidence.json", "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print("EVIDENCE_UNAVAILABLE", type(exc).__name__, str(exc))
