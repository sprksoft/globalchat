const disclaimerEl = document.getElementById("disclaimer");
export const checkbox = document.getElementById("disclaimer-check");
const countdownEl = document.getElementById("disclaimer-countdown");
const countdownTextEl = document.getElementById("disclaimer-countdown-text");

let disclaimerCountdown = 15;
let disclaimerInterval;
if (localStorage.getItem("accepted_disclaimer") == disclaimerEl.dataset.disclaimerVer) {
  checkEnable();
  checkbox.checked = true;
} else {
  checkbox.disabled = true;
  checkbox.checked = false;
  countdownEl.innerText = disclaimerCountdown + (disclaimerCountdown == 1 ? " seconde" : " seconden");
  disclaimerInterval = setInterval(() => {
    disclaimerCountdown--;
    countdownEl.innerText = disclaimerCountdown + (disclaimerCountdown == 1 ? " seconde" : " seconden");

    if (disclaimerCountdown <= 0) {
      checkEnable();
    }
  }, 1000);
}

function checkEnable() {
  if (disclaimerInterval)
    clearInterval(disclaimerInterval);
  checkbox.checked = false;
  checkbox.disabled = false;
  countdownTextEl.innerText="Ga akkoord met de regels om op global chat te kunnen chatten.";
}


checkbox.addEventListener("change", (e) => {
  if (e.target.checked) {
    localStorage.setItem("accepted_disclaimer", disclaimerEl.dataset.disclaimerVer);
  } else {
    localStorage.setItem("accepted_disclaimer", -1);
  }
});

