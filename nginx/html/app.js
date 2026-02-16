async function uploadFile() {
    const fileInput = document.getElementById('csvFile');
    const status = document.getElementById('status');

    if (!fileInput.files[0]) {
        status.textContent = 'Please select a file';
        status.className = 'error';
        return;
    }

    const formData = new FormData();
    formData.append('file', fileInput.files[0]);

    try {
        const response = await fetch('/api/transactions', {
            method: 'PUT',
            body: formData
        });
        const text = await response.text();

        if (response.ok) {
            status.textContent = text;
            status.className = 'success';
            loadTransactions();
        } else {
            status.textContent = 'Error: ' + text;
            status.className = 'error';
        }
    } catch (err) {
        status.textContent = 'Error: ' + err.message;
        status.className = 'error';
    }
}

async function loadTransactions() {
    const tableBody = document.getElementById('tableBody');
    const status = document.getElementById('status');

    try {
        const response = await fetch('/api/transactions');

        if (response.status === 404) {
            tableBody.innerHTML = '<tr><td colspan="6">No transactions uploaded</td></tr>';
            return;
        }

        if (!response.ok) {
            throw new Error(await response.text());
        }

        const data = await response.json();
        tableBody.innerHTML = '';

        if (data.p.length === 0) {
            tableBody.innerHTML = '<tr><td colspan="6">No transactions</td></tr>';
            return;
        }

        data.p.forEach(t => {
            const row = document.createElement('tr');
            row.innerHTML = `
                <td>${t.date}</td>
                <td>${t.symbol}</td>
                <td>${t.number}</td>
                <td>${t.price}</td>
                <td>${t.commision}</td>
                <td>${t.currency}</td>
            `;
            tableBody.appendChild(row);
        });
    } catch (err) {
        status.textContent = 'Error loading: ' + err.message;
        status.className = 'error';
    }
}

// Load on page load
document.addEventListener('DOMContentLoaded', loadTransactions);
