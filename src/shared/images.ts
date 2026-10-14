import { invoke } from "@tauri-apps/api/core";

/**
 * Pasted images. A text field shows each as a label, `[Image #1]`; everything that leaves the
 * field (a saved note, the Pad's draft) has Markdown instead, `![Image #1](images/….png)`,
 * which Obsidian shows as the picture. Each window keeps the paths of its text's images,
 * numbered like the labels, to turn the labels back into Markdown.
 */
export type Images = Map<number, string>;

/** An image as a text field shows it. */
const LABEL = /\[Image #(\d+)\]/g;
/** An image as notes store it: the label, and the path relative to the notes folder. */
const MARKDOWN = /!\[Image #(\d+)\]\(([^()\s]+)\)/g;

const label = (n: number) => `[Image #${n}]`;

/** The number for the next pasted image: one more than the highest so far. */
export function nextImage(images: Images) {
  return Math.max(0, ...images.keys()) + 1;
}

/** `text` with the labels of `images` turned into Markdown; other labels stay as typed. */
export function toMarkdown(text: string, images: Images) {
  return text.replace(LABEL, (typed, n: string) => {
    const path = images.get(Number(n));
    return path === undefined ? typed : `!${typed}(${path})`;
  });
}

/**
 * The text a field shows for `markdown`, its images as labels, and their paths. Two different
 * images with the same number (a note moved into a Pad draft that has images too) get numbers
 * of their own.
 */
export function fromMarkdown(markdown: string): { text: string; images: Images } {
  const images: Images = new Map();
  const text = markdown.replace(MARKDOWN, (_, n: string, path: string) => {
    let number = Number(n);
    const taken = images.get(number);
    if (taken !== undefined && taken !== path) number = nextImage(images);
    images.set(number, path);
    return label(number);
  });
  return { text, images };
}

/** The paths of the images in a note's `markdown`, in order, each once. */
export function imagePaths(markdown: string) {
  return [...new Set([...markdown.matchAll(MARKDOWN)].map((match) => match[2]))];
}

const plain = (text: string): Node[] => [document.createTextNode(text)];

/**
 * `text` as nodes: each match of `pattern` that `show` gives a label for in an accent `.image`
 * span (its content made by `inner`), the text around them made by `other`.
 */
function imageNodes(
  text: string,
  pattern: RegExp,
  show: (match: RegExpMatchArray) => string | undefined,
  other: (text: string) => Node[],
  inner: (label: string) => Node[],
): Node[] {
  const nodes: Node[] = [];
  let from = 0;
  for (const match of text.matchAll(pattern)) {
    const shown = show(match);
    if (shown === undefined) continue;
    if (match.index > from) nodes.push(...other(text.slice(from, match.index)));
    const span = document.createElement("span");
    span.className = "image";
    span.append(...inner(shown));
    nodes.push(span);
    from = match.index + match[0].length;
  }
  if (from < text.length) nodes.push(...other(text.slice(from)));
  return nodes;
}

/**
 * A text field's `text` as nodes, drawn under the field: the labels of `images` marked and kept
 * as they are, the rest made by `other`.
 */
export function labelNodes(text: string, images: Images, other = plain) {
  return imageNodes(
    text,
    LABEL,
    (match) => (images.has(Number(match[1])) ? match[0] : undefined),
    other,
    plain,
  );
}

/**
 * A note's `markdown` as nodes, as the reader and the search results show it: its images as
 * marked labels (made by `inner`), the rest made by `other`.
 */
export function markdownNodes(markdown: string, other = plain, inner = plain) {
  return imageNodes(markdown, MARKDOWN, (match) => label(Number(match[1])), other, inner);
}

/**
 * Pastes images into `field`: an image in the clipboard (with no text, so cells copied from a
 * spreadsheet still paste as text) is saved in the notes folder, added to `images()` and its
 * label typed over the selection, keeping Ctrl+Z working. Anything else pastes as usual.
 */
export function pasteImages(
  field: HTMLTextAreaElement,
  {
    images,
    enabled = () => true,
    onError,
  }: { images: () => Images; enabled?: () => boolean; onError: (err: unknown) => void },
) {
  field.addEventListener("paste", async (e) => {
    const data = e.clipboardData;
    if (!enabled() || !data || data.types.includes("text/plain")) return;
    const file = Array.from(data.files).find((file) => file.type.startsWith("image/"));
    if (!file) return;
    e.preventDefault();
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const path = await invoke<string>("save_image", bytes, {
        headers: { "Image-Type": file.type },
      });
      const map = images();
      const n = nextImage(map);
      map.set(n, path);
      document.execCommand("insertText", false, label(n));
    } catch (err) {
      onError(err);
    }
  });
}
