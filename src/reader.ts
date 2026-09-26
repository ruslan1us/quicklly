import { invoke } from "@tauri-apps/api/core";
import { appWindow, fitToContent, reveal } from "./popup";
import { followTheme } from "./theme";

interface NoteFile {
  name: string;
  notes: number;
}

const rowsList = document.querySelector<HTMLUListElement>("#rows")!;
const empty = document.querySelector<HTMLParagraphElement>("#empty")!;
const error = document.querySelector<HTMLParagraphElement>("#error")!;

/** Beyond this the list scrolls instead of the window growing. */
const MAX_HEIGHT = 520;

let files: NoteFile[] = [];
let selected = 0;

/** Today's daily file name, in local time like the notes themselves. */
function todayFile() {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}.md`;
}

function render() {
  const today = todayFile();
  rowsList.replaceChildren(
    ...files.map((file, i) => {
      const li = document.createElement("li");
      li.className = "row file";
      li.classList.toggle("selected", i === selected);

      const cells = [
        i === selected ? "❯" : "",
        file.name.replace(/^(\d{4}-\d{2}-\d{2})\.md$/, "$1"),
        file.name === today ? "today" : "",
        `${file.notes} ${file.notes === 1 ? "note" : "notes"}`,
      ];
      li.append(
        ...cells.map((text, col) => {
          const span = document.createElement("span");
          span.textContent = text;
          if (col === 3) span.className = "count";
          return span;
        }),
      );
      li.addEventListener("click", () => {
        selected = i;
        render();
      });
      return li;
    }),
  );
  empty.hidden = files.length > 0;
  rowsList.children[selected]?.scrollIntoView({ block: "nearest" });
}

async function load() {
  error.hidden = true;
  try {
    files = await invoke<NoteFile[]>("list_note_files");
  } catch (err) {
    files = [];
    error.textContent = String(err);
    error.hidden = false;
  }
  selected = Math.min(selected, Math.max(files.length - 1, 0));
  render();
  await fitToContent(MAX_HEIGHT);
}

document.addEventListener("keydown", (e) => {
  e.preventDefault();
  // Esc or Ctrl+C (by physical key, so any keyboard layout works) closes like in a terminal.
  if (e.key === "Escape" || (e.ctrlKey && e.code === "KeyC")) {
    void appWindow.close();
    return;
  }
  // → goes back to the note input, the way ← came here.
  if (e.key === "ArrowRight") {
    void invoke("open_input").then(() => appWindow.close());
    return;
  }
  if (files.length === 0) return;
  if (e.key === "ArrowUp") {
    selected = (selected + files.length - 1) % files.length;
    render();
  } else if (e.key === "ArrowDown") {
    selected = (selected + 1) % files.length;
    render();
  }
});

// Notes may have been added since the reader was last shown.
window.addEventListener("focus", () => void load());

// Shown only once themed and filled in, so it never flickers.
void Promise.all([followTheme(), load()]).then(reveal);
