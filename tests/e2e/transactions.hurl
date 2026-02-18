# Test 1: No transactions returns 404
GET {{host}}/api/transactions
HTTP 404

# Test 2: Upload valid CSV returns 200
PUT {{host}}/api/transactions
[MultipartFormData]
file: file,fixtures/valid.csv; text/csv
HTTP 200

# Test 3: GET after upload returns 200 with data
GET {{host}}/api/transactions
HTTP 200
[Asserts]
jsonpath "$.p" count > 0
jsonpath "$.p[0].symbol" == "FOO"

# Test 4: Upload invalid CSV returns 400
PUT {{host}}/api/transactions
[MultipartFormData]
file: file,fixtures/invalid.csv; text/csv
HTTP 400
