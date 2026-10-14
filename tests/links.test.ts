import { describe, expect, test } from "vitest";
import { findLinks, shortLink, splitLinks } from "../src/shared/links";

const urls = (text: string) => findLinks(text).map((link) => link.url);

describe("findLinks", () => {
  test("finds http and https links up to the next whitespace", () => {
    expect(urls("see http://example.com/a?b=1&c=2#d and HTTPS://Example.com/x")).toEqual([
      "http://example.com/a?b=1&c=2#d",
      "HTTPS://Example.com/x",
    ]);
  });

  test("gives where each link is in the text", () => {
    const text = "go to https://example.com now";
    const [link] = findLinks(text);
    expect(link).toEqual({ start: 6, end: 25, url: "https://example.com" });
    expect(text.slice(link.start, link.end)).toBe(link.url);
  });

  test("finds bare domains with a known top-level domain", () => {
    expect(urls("google.com, github.com/ruslan1us and docs.rs.io:8080/path?q")).toEqual([
      "google.com",
      "github.com/ruslan1us",
      "docs.rs.io:8080/path?q",
    ]);
  });

  test("drops sentence punctuation from the end", () => {
    expect(urls("see https://github.com/ruslan1us/quicklly/releases.")).toEqual([
      "https://github.com/ruslan1us/quicklly/releases",
    ]);
    expect(urls("https://example.com/a, https://example.com! google.com?")).toEqual([
      "https://example.com/a",
      "https://example.com",
      "google.com",
    ]);
  });

  test("drops a closing bracket that was not opened inside the link", () => {
    expect(urls("(see https://example.com/a). [https://a.io/c]")).toEqual([
      "https://example.com/a",
      "https://a.io/c",
    ]);
    expect(urls("(https://en.wikipedia.org/wiki/Foo_(bar))")).toEqual([
      "https://en.wikipedia.org/wiki/Foo_(bar)",
    ]);
  });

  test("a bare protocol is not a link", () => {
    expect(urls("https://... and http://")).toEqual([]);
  });

  test.each([
    ["a file name", "notes.md"],
    ["a script", "build.sh"],
    ["a version", "v0.7.4"],
    ["an abbreviation", "e.g. this"],
    ["an e-mail address", "me@site.com"],
    ["localhost", "localhost:3000"],
    ["a file path", "C:/dev/x.md"],
    ["an image path", "images/2026-10-07_14-32-05.png"],
    ["an image in Markdown", "![Image #1](images/2026-10-07_14-32-05.png)"],
  ])("ignores %s", (_, text) => {
    expect(findLinks(text)).toEqual([]);
  });
});

describe("shortLink", () => {
  test("keeps the domain and 5 characters after it", () => {
    expect(shortLink("https://github.com/ruslan1us/quicklly/releases")).toBe("github.com/rusl...");
  });

  test("drops the protocol and leaves short links whole", () => {
    expect(shortLink("https://github.com")).toBe("github.com");
    expect(shortLink("http://github.com/abcd")).toBe("github.com/abcd");
    expect(shortLink("google.com?q=1")).toBe("google.com?q=1");
  });
});

describe("splitLinks", () => {
  test("splits a text into plain parts and links, in order", () => {
    expect(splitLinks("see google.com and https://x.io/a.")).toEqual([
      { text: "see ", link: false },
      { text: "google.com", link: true },
      { text: " and ", link: false },
      { text: "https://x.io/a", link: true },
      { text: ".", link: false },
    ]);
  });

  test("a text without links is one plain part; an empty one has none", () => {
    expect(splitLinks("no links")).toEqual([{ text: "no links", link: false }]);
    expect(splitLinks("")).toEqual([]);
  });
});
