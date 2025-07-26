// Sample JavaScript file for testing directory listing
console.log('Rust HTTP Server - Static Assets Demo');

function greetUser() {
    const name = prompt('What is your name?');
    if (name) {
        alert(`Hello, ${name}! Welcome to our Rust HTTP Server!`);
    }
}

// Simple function to test JS execution
function getCurrentTime() {
    return new Date().toLocaleTimeString();
}

document.addEventListener('DOMContentLoaded', function() {
    console.log('Page loaded at:', getCurrentTime());
});
