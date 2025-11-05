import { gcclient } from "../chat";
import { createMessage, type Message } from "./message";
import { WFTag } from '../gcapi/wf.ts';
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

export function markGood(word: string | HTMLElement) {
  if (word instanceof HTMLElement) {
    word.classList.remove("tag-b", "tag-u");
    word.classList.add("tag-g");
    markGood(word.innerText);
  } else {
    gcclient.wfMarkWord(word, true);
    scheduleCommit();
  }
}

export function markBad(word: string | HTMLElement) {
  if (word instanceof HTMLElement) {
    word.classList.remove("tag-g", "tag-u");
    word.classList.add("tag-b");
    markBad(word.innerText);
  } else {
    gcclient.wfMarkWord(word, false);
    scheduleCommit();
  }
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
