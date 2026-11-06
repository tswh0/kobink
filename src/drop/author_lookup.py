import json, os, re, sys, urllib.error, urllib.request, zipfile

series = sys.argv[1]
sample = sys.argv[2] if len(sys.argv) > 2 else ""

def from_comicinfo(path):
    if not path or not os.path.exists(path):
        return None
    try:
        with zipfile.ZipFile(path) as z:
            name = next((n for n in z.namelist() if n.lower().endswith("comicinfo.xml")), None)
            if not name:
                return None
            xml = z.read(name).decode("utf-8", "ignore")
    except Exception:
        return None
    for tag in ("Writer", "Author"):
        m = re.search(rf"<{tag}>(.*?)</{tag}>", xml, re.S | re.I)
        if m and m.group(1).strip():
            return m.group(1).strip()
    return None

def norm(text):
    return re.sub(r"[^a-z0-9]+", " ", (text or "").lower()).strip()

def plausible(query, media):
    q = norm(query)
    if not q:
        return False
    titles = list((media.get("title") or {}).values()) + (media.get("synonyms") or [])
    for title in titles:
        n = norm(title)
        if not n:
            continue
        if q == n or q in n or n in q:
            return True
        qa, na = set(q.split()), set(n.split())
        if qa and na and len(qa & na) / len(qa | na) >= 0.6:
            return True
    return False

def from_anilist(title):
    query = ("query($s:String){Media(search:$s,type:MANGA)"
             "{title{romaji english}synonyms staff(perPage:15){edges{role node{name{full}}}}}}")
    payload = json.dumps({"query": query, "variables": {"s": title}}).encode()
    req = urllib.request.Request(
        "https://graphql.anilist.co", data=payload,
        headers={"Content-Type": "application/json", "Accept": "application/json",
                 "User-Agent": "kcc-batch/1.1 (manga metadata)"})
    with urllib.request.urlopen(req, timeout=12) as r:
        data = json.load(r)
    media = (data.get("data") or {}).get("Media") or {}
    if not media or not plausible(title, media):
        return None
    edges = ((media.get("staff") or {}).get("edges")) or []

    def role(edge):
        return (edge.get("role") or "").lower()

    pick = next((e for e in edges if "story & art" in role(e)), None) \
        or next((e for e in edges if role(e).startswith("story")), None) \
        or next((e for e in edges if "art" in role(e)), None)
    if not pick:
        return None
    titles = media.get("title") or {}
    matched = titles.get("romaji") or titles.get("english") or "?"
    return pick["node"]["name"]["full"], matched

author = from_comicinfo(sample)
if author:
    print(author)
    print("# source: ComicInfo.xml", file=sys.stderr)
    sys.exit(0)

try:
    got = from_anilist(series)
except urllib.error.HTTPError as exc:
    if exc.code == 404:
        print("# no match on AniList", file=sys.stderr)
    else:
        print(f"# lookup failed: {exc}", file=sys.stderr)
    sys.exit(1)
except Exception as exc:
    print(f"# lookup failed: {exc}", file=sys.stderr)
    sys.exit(1)

if not got:
    print("# no match on AniList", file=sys.stderr)
    sys.exit(1)

print(got[0])
print(f'# source: AniList "{got[1]}"', file=sys.stderr)
