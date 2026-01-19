import './common/common.css'
import './common/buttons.css'
import './common/stat.css'
import './wf_page/index.css'

import { WFEditor } from './common/wfedit.js'
import { getCSRFToken } from './common/utils.js'
import { WFTag } from './gcapi/wf.js'


export let wfEditor = new WFEditor({
  markWord: async (word: string, good: boolean) => {
    let mark = "markgood"
    if (!good) {
      mark = "markbad"
    }

    await fetch("/api/wf/" + encodeURIComponent(word) + "/" + mark, {
      method: "POST",
      headers: {
        "X-CSRF-Protect": getCSRFToken(),
      }
    });
  },

  lock: {
    lockWord: async (word: string, locked: boolean, reason: string) => {
      let lock = "lock"
      if (!locked) {
        lock = "unlock"
      }

      await fetch("/api/wf/" + encodeURIComponent(word) + "/" + lock + "?reason=" + encodeURIComponent(reason), {
        method: "POST",
        headers: {
          "X-CSRF-Protect": getCSRFToken(),
        }
      });
    },
    getLockInfo: async (word: string) => {
      const resp = await fetch("/api/wf/" + encodeURIComponent(word));
      const info = resp.json();

      return info;
    }
  }

});

$("span.editable-word").on("click", async function() {
  await wfEditor.toggle(this)
})

const words = document.querySelectorAll("span.editable-word");
for (let i = 0; i < words.length; i++) {
  const span = words[i] as HTMLElement;
  WFTag.assignToElement(WFTag.fromString(span.getAttribute("data-tag")!), span);
  span.setAttribute("data-tag", "");
}
