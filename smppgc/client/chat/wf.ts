import { socketmgr } from "../chat";
import { createMessage, type Message } from "./mesg";

export enum WFTag {
  Unknown = 0,
  Good = 1,
  Bad = 2,
}
export namespace WFTag {
  export function toString(tag: WFTag): string {
    switch (tag) {
      case WFTag.Unknown: return "unknown";
      case WFTag.Good: return "good";
      case WFTag.Bad: return "bad";
    }
  }
}

export function markGood(word: string | HTMLElement) {
  if (word instanceof HTMLElement) {
    word.classList.remove("bad", "unknown");
    word.classList.add("good");
    markGood(word.innerText);
  } else {
    socketmgr.markWord(word, true);
  }
}

export function markBad(word: string | HTMLElement) {
  if (word instanceof HTMLElement) {
    word.classList.remove("good", "unknown");
    word.classList.add("bad");
    markBad(word.innerText);
  } else {
    socketmgr.markWord(word, false);
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

export function showProfWarn(mesg: Message) {
  countdown = 10;
  const okBtn = $("#profwarn-ok").get(0) as HTMLButtonElement;
  const countdownEl = $("#profwarn-countdown").get(0) as HTMLButtonElement;


  countdownEl.innerText = "10 seconden";
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

  const dialog = $("#profwarn-dialog").get(0) as HTMLDialogElement;
  dialog.showModal();
}

export function clearProfWarn() {
  countdown = 0;
  clearInterval(interval);
  const dialog = $("#profwarn-dialog").get(0) as HTMLDialogElement;
  dialog.close();
}


