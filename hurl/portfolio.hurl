PUT http://localhost:3000/transactions
X-Overwrite: confirm
[MultipartFormData]
transaction_log: file,1trans.csv; text/csv

HTTP 200


GET http://localhost:3000/portfolio
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"
jsonpath "$[0].symbol" == "FOO"
jsonpath "$[0].quantity" == 1
jsonpath "$[0].value" == "1"
jsonpath "$[0].currency" == "BAR"
