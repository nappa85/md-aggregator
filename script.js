let converter = null;
function appendOwner(url, owner) {
    return url + (url.indexOf('?') < 0?'?':'&') + 'owner=' + owner;
}
function open(url, owner) {
    fetch(appendOwner(url, owner))
        .then((response) => response.text())
        .then((data) => {
            const content = converter.makeHtml(data);
            document.querySelector('#content').innerHTML = content;
            document.querySelectorAll('#content a').forEach((self) => {
                const href = self.href;
                if(href && (href.indexOf('http') !== 0 || href.indexOf(window.location.origin) === 0)) {
                    self.setAttribute('href', appendOwner(href, owner));
                }
            });
            document.querySelectorAll('#content img').forEach((self) => {
                const href = self.src;
                if(href && (href.indexOf('http') !== 0 || href.indexOf(window.location.origin) === 0)) {
                    self.setAttribute('src', appendOwner(href, owner));
                }
            });
            document.querySelectorAll('#content pre code').forEach((el) => {
                hljs.highlightElement(el);
            });
            document.querySelector('body').classList.toggle('collapsed');
        });
}
window.addEventListener('load', function() {
    let pathname = decodeURI(window.location.pathname);
    converter = new showdown.Converter();
    document.querySelectorAll('#menu h1').forEach((element) => element.addEventListener('click', function() {
        document.querySelector('body').classList.toggle('collapsed');
    }));
    document.querySelectorAll('#menu li a').forEach((element) => element.addEventListener('click', function(event) {
        const self = event.target;
        event.preventDefault();
        if(self.parentNode.classList.contains('dir')) {
            self.parentNode.classList.toggle('open');
        }
        else {
            document.querySelectorAll('li.active').forEach((self) => self.classList.toggle('active'));
            self.parentNode.classList.toggle('active');
            const url = self.href;
            open(url, self.parentNode.dataset.owner)
            window.history.pushState({}, '', url);
        }
    }));
    document.querySelectorAll('#menu li.dir').forEach((self) => {
        let next = self.nextElementSibling;
        if(next) {
            const url = self.querySelector('a').href;
            if(pathname.indexOf(url) === 0) {
                self.classList.add('open');
            }
            const newParent = document.createElement('ul')
            self.appendChild(newParent);
            while(next && next.querySelector('a').href && next.querySelector('a').href.indexOf(url) === 0) {
                newParent.appendChild(self.nextElementSibling);
                next = self.nextElementSibling;
            }
            if(newParent.querySelectorAll('a').length == 0) {
                self.classList.add('hidden');
            }
        }
        else {
            self.classList.add('hidden');
        }
    });
    document.querySelectorAll('#menu li a[href=\'' + pathname + '\']').forEach((self) => {
        self.parentNode.classList.toggle('active');
        open(pathname, self.parentNode.dataset.owner);
    });
});