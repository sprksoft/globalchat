import { gcclient } from "../chat";
import { createMessage, type Message } from "./message";
import { WFTag } from '../gcapi/wf.ts';
import { WFEditor } from "../common/wf.ts";
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

export let wfEditor = new WFEditor((word, good) => {
  gcclient.wfMarkWord(word, good);
  scheduleCommit();
})

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
