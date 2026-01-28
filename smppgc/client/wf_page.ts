import './common/common.css'
import './common/buttons.css'
import './common/stat.css'
import './wf_page/index.css'

import { WFEditor, type WFEditorConfig } from './common/wfedit.js'
import { getCSRFToken } from './common/utils.js'
import { WFTag } from './gcapi/wf.js'
import { Role } from './gcapi/user.js'

declare const ROLE: Role;

const wfEditorConfig: WFEditorConfig = {
  markWord: async (word: string, good: boolean) => {
    await fetch("/api/wf/" + encodeURIComponent(word) + "/" + (good ? "markgood" : "markbad"), {
      method: "POST",
      headers: {
        "X-CSRF-Protect": getCSRFToken(),
      }
    });
  },
  getWordInfo: async (word: string) => {
    const resp = await fetch("/api/wf/" + encodeURIComponent(word));
    const info = resp.json();

    return info;
  },

  lockWord: undefined
}

if (ROLE >= Role.Admin) {
  wfEditorConfig.lockWord = async (word, locked, reason) => {
    await fetch("/api/wf/" + encodeURIComponent(word) + "/" + (locked ? "lock" : "unlock") + "?reason=" + encodeURIComponent(reason), {
      method: "POST",
      headers: {
        "X-CSRF-Protect": getCSRFToken(),
      }
    });
  }
}

export let wfEditor = new WFEditor(wfEditorConfig);

$("span.editable-word").on("click", async function() {
  await wfEditor.toggle(this)
})

const words = document.querySelectorAll("span.editable-word");
for (let i = 0; i < words.length; i++) {
  const span = words[i] as HTMLElement;
  WFTag.assignToElement(WFTag.fromString(span.getAttribute("data-tag")!), span);
  span.setAttribute("data-tag", "");
}
