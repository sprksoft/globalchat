import $ from "./common/jquery.js";

$(".demote-btn").on("click", async function() {
  await fetch("/api/demote?id=" + this.dataset.uid, { method: "POST" });
  this.parentElement.parentElement.remove();
});

$("#newkey-btn").on("click", async function() {
  await fetch("/api/new_key?role=mod", {
    method: "POST",
  });
  location.reload();
});

$(".copyable").after(function() {
  let key = this.innerText;
  return $("<button class='pillbtn copybtn'>copy</button>").on("click", function() {
    navigator.clipboard.writeText(
      "https://gc.smartschoolplusplus.com/promote?key=" + key,
    );
    this.innerText = "copied!";
    setTimeout(() => {
      this.innerText = "copy";
    }, 1000);
  });
});
