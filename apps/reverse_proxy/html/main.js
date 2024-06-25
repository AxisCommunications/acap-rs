function update_auth() {
    const url = '/local/reverse_proxy/api/admin/whoami';
    fetch(url)
        .then(response => {
            if (!response.ok) {
                throw new Error('Network response was not ok ' + response.statusText);
            }
            return response.text();
        })
        .then(data => {
            document.getElementById('authentication').innerText = 'Authenticated as ' + data;
        })
        .catch(error => {
            document.getElementById('authentication').innerText = 'Error: ' + error;
        });
}

function update_time() {
    const url =
        (window.location.protocol === 'https:' ? 'wss://' : 'ws://') +
        window.location.host +
        '/local/reverse_proxy/api/admin/ws';
    const socket = new WebSocket(url);

    socket.onmessage = function (event) {
        const timestamp = event.data;
        document.getElementById('timestamp').innerText = 'The time is now ' + timestamp;
    };

    socket.onerror = function (error) {
        document.getElementById('timestamp').innerText = 'WebSocket error: ' + error;
    };

    socket.onclose = function () {
        document.getElementById('timestamp').innerText = 'WebSocket connection closed';
    };
}

document.addEventListener('DOMContentLoaded', () => {
    update_auth()
    update_time()
});
