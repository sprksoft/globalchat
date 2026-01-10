import './common/common.css'
import './common/buttons.css'
import './common/wf.css'
import './common/stat.css'
import './wf_page/index.css'

import { WFEditor } from './common/wf.js'
import { getCSRFToken } from './common/utils.js'


export let wfEditor = new WFEditor(async (word: string, good: boolean) => {
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
})

$(".editable-word").on("click", function() {
  wfEditor.toggle(this as HTMLSpanElement)
})
