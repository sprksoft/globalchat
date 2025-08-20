import * as disclaimer from "./login/disclaimer.js";
import { getCSRFToken } from "./common/utils.js";

import "./common/common.css";
import "./common/buttons.css";
import "./common/logo.css";
import "./login/login.css";

$(".oauth-btn").on("click", function () {
  const provider = $(this).attr("data-oauth-provider");

  const url = "/oauth/start?provider=" + provider + "&pses_id=" + PENDING_ID;

  if (INTERNAL_LOGIN) {
    location = url;
  } else {
    window.open(url, "_blank");
    $("#waiting-prompt").get(0).showModal();
  }
});

$("#waiting-prompt").on("close", function () {
  this.showModal();
});

window.addEventListener("message", (e) => {
  if (e.origin == location.origin && e.data.type == "login-complete") {
    location = "/setup_ses/" + PENDING_ID;
  }
});
