const disclaimerEl = document.getElementById("disclaimer");
const checkbox = document.getElementById("disclaimer-check");
const smLoginBtn = document.getElementById("sm-login-btn");
const countdownEl = document.getElementById("disclaimer-countdown");
const countdownTextEl = document.getElementById("disclaimer-countdown-text");

let disclaimerCountdown = 15;

function checkEnable() {
  if (disclaimerInterval)
    clearInterval(disclaimerInterval);
  checkbox.checked = false;
  checkbox.disabled = false;
  countdownTextEl.innerText="Ga akkoord met de regels om op global chat te kunnen chatten.";
}

function updateCountdown(count) {
  countdownEl.innerText = count + (count == 1 ? " seconde" : " seconden");
}

let disclaimerInterval;
if (ENABLE_CHECK) {
  checkEnable();
  checkbox.checked = true;
  smLoginBtn.disabled = false;
} else {
  smLoginBtn.disabled = true;
  checkbox.checked = false;
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
    smLoginBtn.disabled = false;
    document.cookie = "accepted_disclaimer="+disclaimerEl.dataset.disclaimerVer+" ;expires=Wed, 20 May 2026 15:06:13 GMT";
  } else {
    smLoginBtn.disabled = true;
    document.cookie = "accepted_disclaimer=0 ;expires=Wed, 20 May 2026 15:06:13 GMT";
  }
});

