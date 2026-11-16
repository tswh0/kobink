pub fn first_image(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<img")?;
    let end = lower[start..]
        .find('>')
        .map(|offset| start + offset)
        .unwrap_or(lower.len());
    let tag = &lower[start..end];

    for key in ["src=", "xlink:href="] {
        for quote in ['"', '\''] {
            let needle = format!("{key}{quote}");

            if let Some(at) = tag.find(&needle) {
                let from = start + at + needle.len();
                let to = html[from..].find(quote)? + from;

                return Some(html[from..to].to_string());
            }
        }
    }

    None
}

pub fn resolve(base: &str, href: &str) -> String {
    let href = decode(href);

    if href.starts_with('/') {
        return href.trim_start_matches('/').to_string();
    }

    let mut parts: Vec<String> = base.split('/').map(str::to_string).collect();
    parts.pop();

    for segment in href.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(decode(other)),
        }
    }

    parts.join("/")
}

pub fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }

        out.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&out).into_owned()
}
