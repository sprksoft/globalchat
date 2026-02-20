import * as disclaimer from "./login/disclaimer.js";
import { getCSRFToken } from "./common/utils.js";

import "./common/common.css";
import "./common/buttons.css";
import "./common/logo.css";
import "./login/login.css";

let showRetryBtnTimeout = null;

$(".oauth-btn").on("click", function () {
  const provider = $(this).attr("data-oauth-provider");

  const url = "/oauth/start?provider=" + provider + "&pending_id=" + PENDING_ID;

  if (PSES_TYPE === "immediate") {
    location = url;
  } else {
    window.open(url, "_blank", "popup");
    $("#waiting-prompt-retry-btn").hide();
    $("#waiting-prompt").get(0).showModal();
    clearTimeout(showRetryBtnTimeout);
    showRetryBtnTimeout = setTimeout(() => {
      $("#waiting-prompt-retry-btn").show();
    }, 2000);
  }
});

$("#waiting-prompt-retry-btn").on("click", function () {
  $("#waiting-prompt").get(0).close();
});
$("#error-prompt-ok-btn").on("click", function () {
  $("#error-prompt").get(0).close();
});

window.addEventListener("message", (e) => {
  if (e.origin != location.origin) {
    return;
  }

  switch (e.data.type) {
    case "login-complete":
      location = "/setup_ses/" + PENDING_ID;
      break;
    case "login-failed":
      $("#waiting-prompt").get(0).close();
      $("#error-prompt-error").text(e.data.error.toString()); // make sure it's a string to mitigate attacks if login_complete page is compromised.
      $("#error-prompt").get(0).showModal();
      break;
  }
});
