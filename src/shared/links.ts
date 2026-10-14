/**
 * Common top-level domains: a link written without `http(s)://` is only recognised with one of
 * these, so file names (`notes.md`), versions (`v0.7.4`) and the like are not taken for links.
 */
const TLDS = [
  "com", "org", "net", "io", "dev", "app", "ai", "co", "me", "info", "biz", "xyz", "site",
  "online", "tech", "cloud", "page", "blog", "news", "store", "shop", "gg", "ly", "to", "fm",
  "tv", "edu", "gov", "eu", "us", "uk", "ca", "au", "de", "fr", "it", "es", "pt", "nl", "be",
  "ch", "at", "pl", "cz", "sk", "se", "no", "fi", "dk", "ee", "lv", "lt", "ru", "ua", "by",
  "kz", "ge", "jp", "cn", "kr", "in", "br",
  // Left out on purpose, as they are also file extensions: md, sh, cc, am, so.
].join("|");

/**
 * Links in notes, found the same way in every window: `http://` or `https://` then everything
 * up to the next whitespace, or a bare domain like `google.com` or `github.com/ruslan1us`
 * (not inside a word, an e-mail address or a longer path) with an optional port and path.
 */
const LINK = new RegExp(
  String.raw`https?:\/\/\S+|(?<![\w@.\/-])(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+(?:${TLDS})(?![\w-])(?::\d+)?(?:[\/?#]\S*)?`,
  "giu",
);
const PROTOCOL = /^https?:\/\//i;
/** Punctuation that usually ends the sentence around a link rather than the link itself. */
const SENTENCE_END = ".,;:!?";

/** A link in a text: `url` is `text.slice(start, end)`. */
export interface Link {
  start: number;
  end: number;
  url: string;
}

const count = (text: string, char: string) => text.split(char).length - 1;

/**
 * Drops sentence punctuation from the end of a link, and a closing bracket that was not opened
 * inside it, so `(see https://example.com/a).` keeps just the link.
 */
function trimEnd(url: string) {
  for (;;) {
    const last = url[url.length - 1];
    const open = last === ")" ? "(" : last === "]" ? "[" : "";
    if (SENTENCE_END.includes(last) || (open && count(url, open) < count(url, last))) {
      url = url.slice(0, -1);
    } else {
      return url;
    }
  }
}

/** The links in `text`, in order. */
export function findLinks(text: string): Link[] {
  const links: Link[] = [];
  for (const match of text.matchAll(LINK)) {
    const url = trimEnd(match[0]);
    // Nothing left after the protocol: not a link.
    if (url.replace(PROTOCOL, "") === "") continue;
    links.push({ start: match.index, end: match.index + url.length, url });
  }
  return links;
}

/**
 * A link as the reader shows it: without the protocol, and with only the first 5 characters
 * after the domain, so `https://github.com/ruslan1us/quicklly` reads `github.com/rusl...`.
 */
export function shortLink(url: string) {
  const rest = url.replace(PROTOCOL, "");
  const domainEnd = rest.search(/[/?#]/);
  if (domainEnd < 0 || rest.length - domainEnd <= 5) return rest;
  return `${rest.slice(0, domainEnd + 5)}...`;
}

/** `text` split into plain parts and links, in order. */
export function splitLinks(text: string): { text: string; link: boolean }[] {
  const parts: { text: string; link: boolean }[] = [];
  let from = 0;
  for (const link of findLinks(text)) {
    if (link.start > from) parts.push({ text: text.slice(from, link.start), link: false });
    parts.push({ text: link.url, link: true });
    from = link.end;
  }
  if (from < text.length) parts.push({ text: text.slice(from), link: false });
  return parts;
}

/**
 * `text` as nodes, each link in an underlined `.link` span showing `show(url)`: the link itself
 * by default, which keeps the text unchanged where it is drawn under a text area.
 */
export function linkNodes(text: string, show = (url: string) => url): Node[] {
  return splitLinks(text).map((part) => {
    if (!part.link) return document.createTextNode(part.text);
    const span = document.createElement("span");
    span.className = "link";
    span.textContent = show(part.text);
    return span;
  });
}
