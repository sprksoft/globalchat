import { socketmgr } from "../chat";
import { createMessage, Message } from "./mesg";

export enum WFTag {
  Unknown = 0,
  Good = 1,
  Bad = 2,
  Whitespace = 3,
}
export namespace WFTag {
  export function toString(tag: WFTag): string {
    switch (tag) {
      case WFTag.Unknown:
        return "unknown";
      case WFTag.Good:
        return "good";
      case WFTag.Bad:
        return "bad";
      case WFTag.Whitespace:
        return "whitespace";
    }
  }
  export function fromNum(num: number): WFTag {
    if (num < 0 || num > WFTag.Whitespace) {
      console.error("tried to create a WFTag from an out of range number");
      return WFTag.Unknown;
    }
    return num as WFTag;
  }
}

let commitTimeout: number | null = null;
function scheduleCommit() {
  if (commitTimeout !== null) {
    clearTimeout(commitTimeout);
  }

  $("#wfcommit-spinner").show();
  commitTimeout = setTimeout(() => {
    $("#wfcommit-spinner").hide();
    commitTimeout = null;
    socketmgr.wfCommit();
  }, 1000);
}

export function markGood(word: string | HTMLElement) {
  if (word instanceof HTMLElement) {
    word.classList.remove("bad", "unknown");
    word.classList.add("good");
    markGood(word.innerText);
  } else {
    socketmgr.markWord(word, true);
    scheduleCommit();
  }
}

export function markBad(word: string | HTMLElement) {
  if (word instanceof HTMLElement) {
    word.classList.remove("good", "unknown");
    word.classList.add("bad");
    markBad(word.innerText);
  } else {
    socketmgr.markWord(word, false);
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
