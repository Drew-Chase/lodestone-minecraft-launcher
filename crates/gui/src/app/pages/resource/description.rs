//! The addresses in a CurseForge description, made into web addresses the launcher can open or load. CurseForge writes
//! its own pages as paths (`/minecraft/mc-mods/…`) and wraps every outside link in `/linkout?remoteUrl=…`, with the
//! target percent-encoded twice. The target is unwrapped, so a press goes straight there instead of through
//! CurseForge's notice page. Only `http` and `https` addresses come out.

/// The site CurseForge's paths are relative to.
const SITE: &str = "https://www.curseforge.com";
/// The most times a link-out target is decoded. CurseForge encodes it twice.
const DECODE_ROUNDS: usize = 3;

/// A description's link or picture address as a web address, or `None` when it is not one: an in-page anchor, a
/// `mailto:` or `javascript:` link, or a picture written into the page as `data:`.
pub fn resolve(href: &str) -> Option<String> {
    let href = href.trim();
    let absolute = if let Some(rest) = href.strip_prefix("//") {
        format!("https://{rest}")
    } else if href.starts_with('/') {
        format!("{SITE}{href}")
    } else {
        href.to_owned()
    };
    let target = match linkout_query(&absolute) {
        Some(query) => linkout_target(query)?,
        None => absolute,
    };
    is_web(&target).then_some(target)
}

/// The query of a CurseForge link-out address: `https://<…>curseforge.com/linkout?<query>`. `None` for any other
/// address.
fn linkout_query(url: &str) -> Option<&str> {
    let (host, path) = scheme_rest(url)?.split_once('/')?;
    let host = host.to_ascii_lowercase();
    if host != "curseforge.com" && !host.ends_with(".curseforge.com") {
        return None;
    }
    path.strip_prefix("linkout?")
}

/// The web address a link-out's query wraps in `remoteUrl`, decoded until it reads as one. `None` when the query
/// names no target, or the target is not a web address, so a link-out never opens anything else.
fn linkout_target(query: &str) -> Option<String> {
    let mut target = query.split('&').find_map(|pair| pair.strip_prefix("remoteUrl="))?.to_owned();
    for _ in 0..DECODE_ROUNDS {
        if is_web(&target) {
            return Some(target);
        }
        target = percent_decode(&target)?;
    }
    is_web(&target).then_some(target)
}

/// Whether an address is an `http` or `https` address with a host.
fn is_web(url: &str) -> bool {
    scheme_rest(url).is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'))
}

/// What follows `http://` or `https://`, written in any case. `None` for any other scheme.
fn scheme_rest(url: &str) -> Option<&str> {
    ["https://", "http://"].into_iter().find_map(|scheme| {
        url.get(..scheme.len())
            .filter(|prefix| prefix.eq_ignore_ascii_case(scheme))
            .map(|_| &url[scheme.len()..])
    })
}

/// The text with each `%XX` turned into the byte it encodes. `None` when the bytes are not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%'
            && let (Some(high), Some(low)) = (bytes.get(at + 1).and_then(hex_value), bytes.get(at + 2).and_then(hex_value))
        {
            out.push(high * 16 + low);
            at += 3;
            continue;
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).ok()
}

/// The value of a hex digit.
fn hex_value(digit: &u8) -> Option<u8> {
    char::from(*digit).to_digit(16).and_then(|value| u8::try_from(value).ok())
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/resource/description.rs"]
mod tests;
