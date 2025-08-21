// @ts-nocheck
import * as chat from "../chat.js";

export interface Ban {
  expirationTime: Date;
  reason: string;
}

export function parseBan(str): Ban {
  const match = str.match(/^err_banned:([0-9]*):(.*)$/);
  const ban = {
    expirationTime: new Date(parseInt(match[1]) * 1000),
    reason: match[2],
  };
  log(`ban: ${JSON.stringify(ban)}`);
  return ban;
}

function updateReasonFromPreset() {
  let reason = $("#ban-dialog-preset :selected").attr("data-fullreason");
  $("#ban-dialog-reason").val(reason);
  updateReason();
}
$("#ban-dialog-preset").on("change", updateReasonFromPreset);

function updateReason() {
  const dur = $("#ban-dialog-preset :selected").attr("data-duration");
  const len = $("#ban-dialog-reason").val().length;
  $("#ban-dialog-confirm").prop(
    "disabled",
    dur == undefined || len <= 0 || len >= 1000,
  );
}
$("#ban-dialog-reason").on("input", updateReason);

export function showDialog(snowflake, sender) {
  $("#ban-dialog").attr("data-snowflake", snowflake);
  $("#ban-dialog-user").text("@" + sender.username);

  $("#ban-dialog-preset").get(0).selectedIndex = 0;
  updateReasonFromPreset();

  $("#ban-dialog").get(0).showModal();
}

export function reset() {
  $("#ban-dialog").get(0).close();
}

$("#ban-dialog-cancel").on("click", function() {
  $("#ban-dialog").get(0).close();
});

$("#ban-dialog-confirm").on("click", async function() {
  const dur = parseInt($("#ban-dialog-preset :selected").attr("data-duration"));
  const reason = $("#ban-dialog-reason").val();
  const snowflake = BigInt($("#ban-dialog").attr("data-snowflake"));
  await chat.socketmgr.banMessageAuthor(snowflake, dur, reason);

  $("#ban-dialog").get(0).close();
});

function secondsToString(sec) {
  const SEC_DAY = 24 * 60 * 60;
  const SEC_HOUR = 60 * 60;
  const SEC_MIN = 60;
  if (sec > SEC_DAY) {
    const days = Math.ceil(sec / SEC_DAY);
    return days == 1 ? "1 dag" : days + " dagen";
  } else if (sec > SEC_HOUR) {
    const hour = Math.ceil(sec / SEC_HOUR);
    return hour + " uur";
  } else if (sec > SEC_MIN) {
    const min = Math.ceil(sec / SEC_MIN);
    return min == 1 ? "1 minuut" : min + " minuten";
  } else {
    return sec == 1 ? "1 seconde" : Math.ceil(sec) + " seconden";
  }
}
export function setBan(ban: Ban) {
  let secsLeft = (ban.expirationTime.getTime() - new Date().getTime()) / 1000;
  $("#ban-reason").text(ban.reason);
  $("#ban-release-time").text(secondsToString(secsLeft));
  $("#ban").show();

  setInterval(() => {
    if (secsLeft <= 0) {
      location.reload();
    }
    secsLeft--;
    $("#ban-release-time").text(secondsToString(secsLeft));
  }, 1000);
}
