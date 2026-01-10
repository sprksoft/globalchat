import './wf.css'

export class WFEditor {
  #mark: (word: string, good: boolean) => void

  constructor(markWord: (word: string, good: boolean) => void) {
    this.#mark = markWord;
  }

  markGood(word: string | HTMLElement) {
    if (word instanceof HTMLElement) {
      word.classList.remove("tag-b", "tag-u");
      word.classList.add("tag-g");
      this.markGood(word.innerText);
    } else {
      this.#mark(word, true);
    }
  }

  markBad(word: string | HTMLElement) {
    if (word instanceof HTMLElement) {
      word.classList.remove("tag-g", "tag-u");
      word.classList.add("tag-b");
      this.markBad(word.innerText);
    } else {
      this.#mark(word, false);
    }
  }

  toggle(word: HTMLSpanElement) {
    let w = $(word)
    if (w.hasClass("tag-u")) {
      this.markGood(word);
    } else if (w.hasClass("tag-g")) {
      this.markBad(word);
    } else if (w.hasClass("tag-b")) {
      this.markGood(word);
    }

  }
}


