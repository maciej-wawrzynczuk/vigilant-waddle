GET http://localhost:3000/transactions
HTTP 200
[Asserts]
jsonpath "$" count == 0
