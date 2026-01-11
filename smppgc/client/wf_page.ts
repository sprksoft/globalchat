import './common/common.css'
import './common/buttons.css'
import './common/stat.css'
import './wf_page/index.css'

import { WFEditor } from './common/wfedit.js'
import { getCSRFToken } from './common/utils.js'


export let wfEditor = new WFEditor({
  markWord: async (word: string, good: boolean) => {
    let mark = "markgood"
    if (!good) {
      mark = "markbad"
    }

    fetch("/api/wf/" + encodeURIComponent(word) + "/" + mark, {
      method: "POST",
      headers: {
        "X-CSRF-Protect": getCSRFToken(),
      }
    }).catch((reason) => alert("Failed to call api:\n" + reason))
  },

  lock: {
    lockWord: (word: string, locked: boolean, reason: string) => {
      console.log("lock " + locked, reason);
    },
    getLockInfo: (word: string) => {
      return { reason: "hello" };
    }
  }

});

$("span.editable-word").on("click", function() {
  wfEditor.toggle(this)
})
