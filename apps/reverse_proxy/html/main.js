async function update_auth() {
    const requested_policy = document.getElementById('requested_policy').value
    const url = `/local/reverse_proxy/api/${requested_policy}/whoami`;
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error('Network response was not ok ' + response.statusText);
    }
    const confirmed_policy = await response.text();
    document.getElementById('confirmed_policy').innerText = `Confirmed policy: ${confirmed_policy}.`;
}

function update_time() {
    const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws';
    const host = window.location.host;
    const url = `${protocol}://${host}/local/reverse_proxy/api/anonymous/ws`;
    const socket = new WebSocket(url);

    socket.onmessage = function (event) {
        const timestamp = event.data;
        document.getElementById('timestamp').innerText = `The time is now ${timestamp}.`;
    };

    socket.onerror = function (error) {
        console.log(error)
    };

    socket.onclose = function () {
        console.log("WebSocket closed")
    };
}

document.addEventListener('DOMContentLoaded', update_time);
