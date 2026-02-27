PUT http://localhost:3000/transactions
[MultipartFormData]
transaction_log: file,1trans.csv; text/csv

HTTP 200


GET http://localhost:3000/transactions
HTTP 200
[Asserts]
jsonpath "$.[0].symbol" == "FOO"
