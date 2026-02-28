PUT http://localhost:3000/transactions
X-Overwrite: confirm
[MultipartFormData]
transaction_log: file,1trans.csv; text/csv

HTTP 200


GET http://localhost:3000/portfolio
HTTP 200
[Asserts]
header "Content-Type" contains "application/yaml"
body contains "FOO: 1"
