import { gcclient } from "../chat";
import { createMessage, type Message } from "./message";
import { WFTag } from '../gcapi/wf.ts';
import { WFEditor, type WFEditorConfig } from "../common/wfedit.ts";
import { getCSRFToken } from "../common/utils.ts";
import { Role } from "../gcapi/user.ts";
export { WFTag };


let commitTimeout: number | null = null;
function scheduleCommit() {
  if (commitTimeout !== null) {
    clearTimeout(commitTimeout);
  }

  $("#wfcommit-spinner").show();
  commitTimeout = setTimeout(() => {
    $("#wfcommit-spinner").hide();
    commitTimeout = null;
    gcclient.wfCommit();
  }, 1000);
}


export function setupWFEditor(role: Role): WFEditor | null {
  if (role <= Role.User) {
    return null;
  }

  const wfEditorConfig: WFEditorConfig = {
    markWord: async (word, good) => {
      gcclient.wfMarkWord(word, good);
      scheduleCommit();
    },
    getWordInfo: async (word) => {
      const resp = await fetch("/api/wf/" + encodeURIComponent(word));
      const info = resp.json();

      return info;
    },
    lockWord: undefined
  }

  if (role >= Role.Admin) {
    wfEditorConfig.lockWord = async (word, locked, reason) => {
      if (locked) {
        gcclient.wfLock(word, reason);
      } else {
        gcclient.wfUnlock(word);
      }
    }
  }
  return new WFEditor(wfEditorConfig);
}


let countdown = 0;
let interval: number;

export function setupProfWarn() {
  $("#profwarn-dialog").on("close", function() {
    if (countdown > 0) {
      const dialog = $("#profwarn-dialog").get(0) as HTMLDialogElement;
      dialog.showModal();
    }
  });

  $("#profwarn-ok").on("click", function() {
    const dialog = $("#profwarn-dialog").get(0) as HTMLDialogElement;
    dialog.close();
  });
}

export function showProfWarn(mesg: Message, time = 10) {
  const dialog = $("#profwarn-dialog").get(0) as HTMLDialogElement;
  if (dialog.open) {
    console.error("prof dialog already open");
    return;
  }

  countdown = time;
  const okBtn = $("#profwarn-ok").get(0) as HTMLButtonElement;
  const countdownEl = $("#profwarn-countdown").get(0) as HTMLButtonElement;

  countdownEl.innerText = time + " seconden";
  okBtn.disabled = true;
  $("#profwarn-message").empty();
  $("#profwarn-message").append(createMessage(mesg, [], false));

  interval = setInterval(() => {
    countdown--;
    countdownEl.innerText =
      countdown + (countdown == 1 ? " seconde" : " seconden");

    if (countdown == 0) {
      clearInterval(interval);
      okBtn.innerText = "Ok";
      okBtn.disabled = false;
    }
  }, 1000);

  dialog.showModal();
}

export function clearProfWarn() {
  countdown = 0;
  clearInterval(interval);
  const dialog = $("#profwarn-dialog").get(0) as HTMLDialogElement;
  dialog.close();
}
