$ErrorActionPreference = 'Stop'
# Read-only checks against the existing isolated fixture. Never seeds or updates orders.
$base = 'http://127.0.0.1:15441'
$tenant = 'a1100000-0000-4000-8000-000000000001'
$login = Invoke-RestMethod -Method Post -Uri "$base/auth/login" -ContentType 'application/json' -Body (@{
    tenant_id = $tenant
    email = 'qa.acceptance@example.invalid'
    password = 'ErpQa!2026'
} | ConvertTo-Json)
$headers = @{ Authorization = "Bearer $($login.access_token)" }
$cases = @(
    @{ query = 'total_amount_min_cent=1250&total_amount_max_cent=1250&limit=10&offset=0'; total = 12; count = 10 },
    @{ query = 'total_amount_min_cent=1250&total_amount_max_cent=1250&limit=10&offset=10'; total = 12; count = 2 },
    @{ query = 'total_amount_max_cent=1249'; total = 0; count = 0 },
    @{ query = 'total_amount_min_cent=1251'; total = 0; count = 0 }
)
$results = foreach ($case in $cases) {
    $result = Invoke-RestMethod -Uri "$base/v1/tenants/$tenant/orders?$($case.query)" -Headers $headers
    if ($result.total -ne $case.total -or @($result.items).Count -ne $case.count) {
        throw "Unexpected order amount result: $($case.query)"
    }
    [pscustomobject]@{ query = $case.query; total = $result.total; count = @($result.items).Count }
}
$rejected = foreach ($query in @('total_amount_min_cent=1251&total_amount_max_cent=1250', 'total_amount_min_cent=-1', 'total_amount_min_cent=invalid')) {
    $response = Invoke-WebRequest -Uri "$base/v1/tenants/$tenant/orders?$query" -Headers $headers -SkipHttpErrorCheck
    if ($response.StatusCode -ne 400) { throw "Expected 400 for $query, got $($response.StatusCode)" }
    [pscustomobject]@{ query = $query; status = $response.StatusCode }
}
@{ matched = $results; rejected = $rejected } | ConvertTo-Json -Depth 5
