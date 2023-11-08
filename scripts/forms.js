htmx.defineExtension("form-validation", {
  onEvent: function (name, evt) {
    if (name !== "htmx:configRequest") return;
    var form = evt.target.closest("form");
    if (!form) return;
    var valid = form.checkValidity();
    if (valid) return;

    evt.preventDefault();
    form.classList.add("was-validated");
  },
});
