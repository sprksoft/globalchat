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
  getWordInfo: GetWordInfoFn | undefined,
  lockWord: LockWordFn | undefined
}

export class WFEditor {
  #mark: MarkWordFn;
  #getWordInfo: GetWordInfoFn | undefined;
  #currentlyEditingWord: HTMLElement | undefined;

  lockMode: boolean = false;

  constructor(conf: WFEditorConfig) {
    this.#mark = conf.markWord;
    this.#getWordInfo = conf.getWordInfo;

    if (conf.lockWord || conf.getWordInfo) {
      const lockEditDialog = document.getElementById("wf-lockedit") as HTMLDialogElement | undefined;
      if (!lockEditDialog) {
        console.error("BUG: The file: wfedit.html.tera needs to be included in the html code for the wf-lockedit dialog to work");
        return;
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
        this.#currentlyEditingWord = undefined;
      });


      $("#wf-lockedit-dialog-cancel").on("click", function() {
        lockEditDialog.close();
      });
      $("#wf-lockedit-reason").prop("disabled", conf.lockWord == undefined);
      $("#wf-lockedit-dialog-lock").prop("disabled", conf.lockWord == undefined);
      $("#wf-lockedit-dialog-unlock").prop("disabled", conf.lockWord == undefined);

      if (conf.lockWord) {
        const confirm = async (locked: boolean) => {
          if (!this.#currentlyEditingWord) { return; }
          this.#currentlyEditingWord.classList.remove("locked");
          if (locked) {
            this.#currentlyEditingWord.classList.add("locked");
          }
          await conf.lockWord!(this.#currentlyEditingWord.innerText, locked, $("#wf-lockedit #wf-lockedit-reason").val()!.toString());
        }

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

  /*
   * Edit/view lockdata of a word
  */
  async lockeditWord(word: HTMLElement) {
    if (!this.#getWordInfo) {
      console.error("Tried to lockedit a word but no getWordInfo handler was provided.");
      return;
    }

    const wordStr = word.innerText;
    const info = await this.#getWordInfo(wordStr);
    this.#currentlyEditingWord = word;
    $("#wf-lockedit #wf-lockedit-word").text(wordStr);
    $("#wf-lockedit #wf-lockedit-reason").val(info.lock_reason);
    ($("#wf-lockedit").get(0) as HTMLDialogElement).showModal();
  }

  async toggle(word: HTMLSpanElement) {
    let w = $(word)
    if (this.lockMode || w.hasClass("locked")) {
      await this.lockeditWord(word)
      return;
    }

    if (w.hasClass("tag-u") || w.hasClass("tag-b")) {
      this.markWord(word, true);
    } else if (w.hasClass("tag-g")) {
      this.markWord(word, false);
    }

  }
}

