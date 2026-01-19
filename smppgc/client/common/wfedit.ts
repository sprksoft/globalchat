import './wf.css'
import './wfedit.css'

export type MarkWordFn = (word: string, good: boolean) => Promise<void>;
export type LockWordFn = (word: string, locked: boolean, reason: string) => Promise<void>;
export type GetWordInfoFn = (word: string) => Promise<WordInfo>;

export type WordInfo = {
  lock_reason: string,
}

export type WFEditorConfig = {
  markWord: MarkWordFn,
  getWordInfo: GetWordInfoFn,
  lockWord: LockWordFn | undefined
}

export class WFEditor {
  #mark: MarkWordFn;
  #getWordInfo: GetWordInfoFn | undefined;
  #lockEditWord: HTMLElement | undefined;

  lockMode: boolean = false;

  constructor(conf: WFEditorConfig) {
    this.#mark = conf.markWord;
    this.#getWordInfo = conf.getWordInfo;

    if (conf.lockWord) {
      const lockEditDialog = document.getElementById("wf-lockedit") as HTMLDialogElement | undefined;
      if (!lockEditDialog) {
        console.error("BUG: The file: wfedit.html.tera needs to be included in the html code for the wf-lockedit dialog to work");
        return;
      }

      const lockWord = conf.lock.lockWord;

      const confirm = async (locked: boolean) => {
        if (!this.#lockEditWord) { return; }
        this.#lockEditWord.classList.remove("locked");
        if (locked) {
          this.#lockEditWord.classList.add("locked");
        }
        await lockWord!(this.#lockEditWord.innerText, locked, $("#wf-lockedit #wf-lockedit-reason").val()!.toString());
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

      $("#wf-lockedit-dialog-lock").on("click", async () => {
        await confirm(true);
        lockEditDialog.close();
      });

      $("#wf-lockedit-dialog-unlock").on("click", async () => {
        await confirm(false);
        lockEditDialog.close();
      });


    }
  }

  async markWord(word: string | HTMLElement, good: boolean) {
    if (word instanceof HTMLElement) {
      word.classList.remove("tag-g", "tag-u", "tag-b");
      if (good) {
        word.classList.add("tag-g");
      } else {
        word.classList.add("tag-b");
      }
      await this.markWord(word.innerText, good);
    } else {
      await this.#mark(word, false);
    }
  }

  async lockeditWord(word: HTMLElement) {
    if (!this.#getWordInfo) { return; }

    const wordStr = word.innerText;
    const info = await this.#getWordInfo(wordStr);
    this.#lockEditWord = word;
    $("#wf-lockedit #wf-lockedit-word").text(wordStr);
    $("#wf-lockedit #wf-lockedit-reason").val(info.lock_reason);
    ($("#wf-lockedit").get(0) as HTMLDialogElement).showModal();
  }

  async toggle(word: HTMLSpanElement) {
    let w = $(word)
    if (this.lockMode) {
      await this.lockeditWord(word)
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

