const form = document.querySelector('form');
const input = form.querySelector('input[type="file"]');
const profile = form.querySelector('select[name="profile"]');
const button = form.querySelector('button');
const bar = document.querySelector('.bar');
const fill = bar.querySelector('i');
const phase = document.querySelector('.phase');
const results = document.querySelector('.results');

form.addEventListener('submit', (event) => {
  if (!window.FormData || !input.files.length) return;
  event.preventDefault();

  const files = Array.from(input.files);
  const total = files.length;

  results.replaceChildren();
  bar.hidden = false;
  button.disabled = true;

  const progress = (fraction, label) => {
    const percent = Math.min(100, fraction * 100);
    fill.style.width = percent.toFixed(1) + '%';
    bar.setAttribute('aria-valuenow', String(Math.round(percent)));
    phase.textContent = label || '';
    phase.hidden = !label;
  };

  const entry = (name, text, ok) => {
    const item = document.createElement('li');
    const title = document.createElement('span');
    const state = document.createElement('em');
    title.textContent = name;
    state.textContent = text;
    if (!ok) state.className = 'error';
    item.append(title, state);
    results.append(item);
  };

  const next = (index) => {
    if (index === total) {
      progress(1, '');
      button.disabled = false;
      input.value = '';
      setTimeout(() => {
        bar.hidden = true;
        fill.style.width = '0';
      }, 1400);
      return;
    }

    const file = files[index];
    const body = new FormData();
    body.append('file', file);
    if (profile) body.append('profile', profile.value);

    const request = new XMLHttpRequest();
    request.open('POST', form.action);

    request.upload.addEventListener('progress', (event) => {
      if (!event.lengthComputable) return;
      progress((index + (event.loaded / event.total) * 0.85) / total, 'Uploading ' + file.name);
    });

    request.upload.addEventListener('load', () => {
      progress((index + 0.85) / total, 'Converting ' + file.name);
    });

    request.addEventListener('load', () => {
      const page = new DOMParser().parseFromString(request.responseText, 'text/html');
      const notice = page.querySelector('.notice');
      const ok = Boolean(notice && notice.classList.contains('ok'));

      entry(file.name, notice ? notice.textContent : 'The answer could not be read.', ok);
      progress((index + 1) / total, '');
      next(index + 1);
    });

    request.addEventListener('error', () => {
      entry(file.name, 'The upload failed.', false);
      next(index + 1);
    });

    request.send(body);
  };

  next(0);
});
