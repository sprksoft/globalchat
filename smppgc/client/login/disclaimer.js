const disclaimerEl = document.getElementById("disclaimer");
const checkbox = document.getElementById("disclaimer-check");
const countdownEl = document.getElementById("disclaimer-countdown");
const countdownTextEl = document.getElementById("disclaimer-countdown-text");

let disclaimerCountdown = 15;

function checkEnable() {
  if (disclaimerInterval)
    clearInterval(disclaimerInterval);
  checkbox.disabled = false;
  countdownTextEl.innerText="Ga akkoord met de regels om op global chat te kunnen chatten.";
}

function updateCountdown(count) {
  countdownEl.innerText = count + (count == 1 ? " seconde" : " seconden");
}

function setAccepted(accepted) {
  if (accepted) {
    checkEnable();
  }
  $("#disclaimer-check").prop("checked", accepted);
  $(".oauth-btns *").prop("disabled", !accepted);
}

let disclaimerInterval;
if (ENABLE_CHECK) {
  setAccepted(true);
} else {
  setAccepted(false);
  checkbox.disabled = true;
  updateCountdown(disclaimerCountdown);
  disclaimerInterval = setInterval(() => {
    disclaimerCountdown--;
    updateCountdown(disclaimerCountdown);

    if (disclaimerCountdown <= 0) {
      checkEnable();
    }
  }, 1000);
}

checkbox.addEventListener("change", (e) => {
  if (e.target.checked) {
    setAccepted(true);
    document.cookie = "accepted_disclaimer="+disclaimerEl.dataset.disclaimerVer+" ;expires=Wed, 20 May 2026 15:06:13 GMT";
  } else {
    setAccepted(false);
    document.cookie = "accepted_disclaimer=0 ;expires=Wed, 20 May 2026 15:06:13 GMT";
  }
});

