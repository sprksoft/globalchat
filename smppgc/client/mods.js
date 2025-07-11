import $ from "./common/jquery.js";
import {getCSRFToken} from './common/utils.js';

import './common/common.css'
import './common/buttons.css'
import './common/copybtn.css'

$(".demote-btn").on("click", async function() {
  await fetch("/api/demote?id=" + this.dataset.uid, { method: "POST", headers: {
    "X-CSRF-Protect": getCSRFToken(),
  } });
  this.parentElement.parentElement.remove();
});

$(".new-key-btn").on("click", async function() {
  await fetch("/api/new_key?role="+$(this).attr("data-role"), {
    method: "POST",
    headers: {
      "X-CSRF-Protect": getCSRFToken(),
    }
  });
  location.reload();
})

$(".copyable").after(function() {
  let key = this.innerText;
  return $("<button class='pillbtn copybtn'>copy</button>").on("click", function() {
    navigator.clipboard.writeText(
      location.origin+"/promote?key=" + key,
    );
    this.innerText = "copied!";
    setTimeout(() => {
      this.innerText = "copy";
    }, 1000);
  });
});

