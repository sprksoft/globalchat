import * as disclaimer from './login/disclaimer.js'
import { getCSRFToken } from './common/utils.js';
import $ from './common/jquery.js'

import './common/common.css'
import './common/buttons.css'
import './common/logo.css'
import './login/login.css'

let isFocused = true;
window.addEventListener("blur", function () {
  isFocused = false;
});
window.addEventListener("focus", function () {
  isFocused = true;
});

$(".oauth-btn").on("click", function () {
  const provider = $(this).attr("data-oauth-provider");
  window.open('/oauth/start?provider='+provider, "_blank");
  $("#waiting-prompt").get(0).showModal();
  setInterval(async () => {
    if (isFocused) {
      console.log("api call");
      const res = await fetch("/api/login/poll", {
        headers: {
          "X-CSRF-Protect": getCSRFToken(),
        }
      });
      if (res.status == 200) {
        location.href = await res.text();
      }
    }

  }, 5000);
});


