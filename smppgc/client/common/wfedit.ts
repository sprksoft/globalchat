import './wf.css'
import './wfedit.css'

export type MarkWordFn = (word: string, good: boolean) => void;
export type LockWordFn = ((word: string, locked: boolean, reason: string) => void);
export type GetLockInfoFn = ((word: string) => LockInfo);

export type LockInfo = {
  reason: string,
}

export type WFEditorConfig = {
  markWord: MarkWordFn,
  lock: { lockWord: LockWordFn, getLockInfo: GetLockInfoFn } | undefined
}

export class WFEditor {
  #mark: MarkWordFn;
  #getLockInfo: GetLockInfoFn | undefined;
  #lockEditWord: HTMLElement | undefined;

  lockMode: boolean = false;

  constructor(conf: WFEditorConfig) {
    this.#mark = conf.markWord;

    if (conf.lock) {
      const lockEditDialog = document.getElementById("wf-lockedit") as HTMLDialogElement | undefined;
      if (!lockEditDialog) {
        console.error("BUG: The file: wfedit.html.tera needs to be included in the html code for the wf-lockedit dialog to work");
        return;
      }

      this.#getLockInfo = conf.lock.getLockInfo;
      const lockWord = conf.lock.lockWord;

      const confirm = (locked: boolean) => {
        if (!this.#lockEditWord) { return; }
        this.#lockEditWord.classList.remove("locked");
        if (locked) {
          this.#lockEditWord.classList.add("locked");
        }
        lockWord!(this.#lockEditWord.innerText, locked, $("#wf-lockedit #wf-lockedit-reason").val()!.toString());
      }

      document.addEventListener("keydown", (e) => {
        if (e.key === "Control") {
          this.lockMode = true;
          document.body.classList.add("wf-lockmode");
        }
      });
      document.addEventListener("keyup", (e) => {
        if (e.key === "Control") {
          this.lockMode = false;
          document.body.classList.remove("wf-lockmode");
        }
      });

      lockEditDialog.addEventListener("close", () => {
        this.#lockEditWord = undefined;
      });


      $("#wf-lockedit-dialog-cancel").on("click", function() {
        lockEditDialog.close();
      });

      $("#wf-lockedit-dialog-lock").on("click", () => {
        confirm(true);
        lockEditDialog.close();
      });

      $("#wf-lockedit-dialog-unlock").on("click", () => {
        confirm(false);
        lockEditDialog.close();
      });


    }
  }

  markWord(word: string | HTMLElement, good: boolean) {
    if (word instanceof HTMLElement) {
      word.classList.remove("tag-g", "tag-u", "tag-b");
      if (good) {
        word.classList.add("tag-g");
      } else {
        word.classList.add("tag-b");
      }
      this.markWord(word.innerText, good);
    } else {
      this.#mark(word, false);
    }
  }

  lockeditWord(word: HTMLElement) {
    if (!this.#getLockInfo) { return; }

    const wordStr = word.innerText;
    const info = this.#getLockInfo(wordStr);
    this.#lockEditWord = word;
    $("#wf-lockedit #wf-lockedit-word").text(wordStr);
    $("#wf-lockedit #wf-lockedit-reason").val(info.reason);
    ($("#wf-lockedit").get(0) as HTMLDialogElement).showModal();
  }

  toggle(word: HTMLSpanElement) {
    let w = $(word)
    if (this.lockMode) {
      this.lockeditWord(word)
      return;
    }
    if (w.hasClass("locked")) {
      return;
    }

    if (w.hasClass("tag-u") || w.hasClass("tag-b")) {
      this.markWord(word, true);
    } else if (w.hasClass("tag-g")) {
      this.markWord(word, false);
    }

  }
}

