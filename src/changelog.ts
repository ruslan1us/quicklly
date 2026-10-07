import { getVersion } from "@tauri-apps/api/app";
import changelog from "../CHANGELOG.md?raw";
import { appWindow, fitToContent, reveal } from "./popup";
import { followTheme } from "./theme";

const header = document.querySelector<HTMLElement>("header")!;
const releasesList = document.querySelector<HTMLElement>("#releases")!;
const footer = document.querySelector<HTMLElement>("#footer")!;

/** Beyond this the changelog scrolls instead of the window growing. */
const MAX_HEIGHT = 560;
const SCROLL_STEP = 44;

/** A line of a release: an `### Added`-style group name, an entry, or plain text. */
interface Block {
  kind: "group" | "entry" | "text";
  text: string;
}

interface Release {
  title: string;
  blocks: Block[];
}

/**
 * Reads the releases out of CHANGELOG.md (Keep a Changelog format). The intro and the link
 * definitions at the bottom are left out, and inline Markdown stays plain text.
 */
function parseChangelog(markdown: string): Release[] {
  const releases: Release[] = [];
  let release: Release | undefined;
  // The block that an indented (wrapped) line belongs to; a blank line ends it.
  let last: Block | undefined;
  for (const line of markdown.split(/\r?\n/)) {
    const heading = /^## \[([^\]]+)\](?: - (.+))?/.exec(line);
    if (heading) {
      const [, version, date] = heading;
      release = { title: date ? `${version} · ${date}` : version, blocks: [] };
      releases.push(release);
      last = undefined;
      continue;
    }
    // Text before the first release is the intro; "[0.7.4]: https://…" are the links.
    if (!release || /^\[[^\]]+\]: /.test(line)) continue;
    const text = line.trim().replace(/`/g, "");
    if (!text) {
      last = undefined;
    } else if (line.startsWith("### ")) {
      last = undefined;
      release.blocks.push({ kind: "group", text: text.slice(4) });
    } else if (line.startsWith("- ")) {
      last = { kind: "entry", text: text.slice(2) };
      release.blocks.push(last);
    } else if (last) {
      last.text += ` ${text}`;
    } else {
      last = { kind: "text", text };
      release.blocks.push(last);
    }
  }
  // Unreleased is only worth showing once something is in it.
  return releases.filter(
    (r) => r.title !== "Unreleased" || r.blocks.some((b) => b.kind === "entry"),
  );
}

function renderRelease(release: Release): HTMLElement[] {
  const title = document.createElement("h2");
  title.textContent = release.title;
  const nodes: HTMLElement[] = [title];
  let list: HTMLUListElement | undefined;
  for (const block of release.blocks) {
    if (block.kind === "entry") {
      if (!list) {
        list = document.createElement("ul");
        nodes.push(list);
      }
      const item = document.createElement("li");
      item.textContent = block.text;
      list.append(item);
    } else {
      list = undefined;
      const node = document.createElement(block.kind === "group" ? "h3" : "p");
      node.textContent = block.text;
      nodes.push(node);
    }
  }
  return nodes;
}

document.addEventListener("keydown", (e) => {
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    e.preventDefault();
    void appWindow.close();
  } else if (e.key === "ArrowUp" || e.key === "ArrowDown") {
    e.preventDefault();
    // The body is what scrolls (see terminal.css).
    document.body.scrollBy(0, e.key === "ArrowUp" ? -SCROLL_STEP : SCROLL_STEP);
  }
});

async function load() {
  const version = await getVersion();
  header.textContent = `Quicklly · Changelog · v${version}`;
  releasesList.replaceChildren(...parseChangelog(changelog).flatMap(renderRelease));
  footer.textContent = "↑↓ scroll · Esc close";
  await fitToContent(MAX_HEIGHT);
}

// The zoom has to be known before the window is fitted to its content.
void followTheme(() => void fitToContent(MAX_HEIGHT))
  .then(load)
  .then(reveal);
