$ErrorActionPreference = 'Stop'
# Read-only checks against the isolated SKU fill fixture. Never updates products or SKUs.
$base = 'http://127.0.0.1:15441'
$tenant = 'a1100000-0000-4000-8000-000000000001'
$parent = 'a1400000-0000-4000-8000-000000000001'
$otherParent = 'a1400000-0000-4000-8000-000000000002'

$login = Invoke-RestMethod -Method Post -Uri "$base/auth/login" -ContentType 'application/json' -Body (@{
    tenant_id = $tenant
    email = 'qa.acceptance@example.invalid'
    password = 'ErpQa!2026'
} | ConvertTo-Json)
$headers = @{ Authorization = "Bearer $($login.access_token)" }

function New-Query([array]$sorts = @(), [hashtable]$filters = @{}) {
    @{
        schema = 1
        offset = 0
        limit = 1
        filters = $filters
        sorts = $sorts
        search = $null
        expression = $null
    }
}

function Invoke-Ranges([string]$product, [hashtable]$query, [array]$ranges) {
    $body = @{ query = $query; ranges = $ranges } | ConvertTo-Json -Depth 10 -Compress
    Invoke-RestMethod -Method Post -Uri "$base/v1/tenants/$tenant/products/$product/skus/query-ranges" -Headers $headers -ContentType 'application/json' -Body $body
}

function Assert-Codes($response, [array]$expected, [string]$case) {
    $actual = @($response.pages | ForEach-Object { $_.data } | ForEach-Object { $_.sku_code })
    if (($actual -join ',') -ne ($expected -join ',')) {
        throw "$case returned [$($actual -join ',')], expected [$($expected -join ',')]"
    }
}

$default = Invoke-Ranges $parent (New-Query) @(
    @{ offset = 0; limit = 1 },
    @{ offset = 10; limit = 2 }
)
if ($default.total -ne 24 -or @($default.pages).Count -ne 2) {
    throw "Default SKU range query expected total=24 and two pages"
}
Assert-Codes $default @('QA-SF-024', 'QA-SF-014', 'QA-SF-013') 'default order'

$codeAsc = Invoke-Ranges $parent (New-Query @(@{ column = 0; descending = $false })) @(
    @{ offset = 0; limit = 1 },
    @{ offset = 10; limit = 2 }
)
if ($codeAsc.total -ne 24) { throw "Code ascending query expected total=24" }
Assert-Codes $codeAsc @('QA-SF-001', 'QA-SF-011', 'QA-SF-012') 'code ascending'

$cost = Invoke-Ranges $parent (New-Query -sorts @() -filters @{ '4' = @{ Money = @{ min = 2400; max = $null } } }) @(
    @{ offset = 0; limit = 1 },
    @{ offset = 9; limit = 2 }
)
if ($cost.total -ne 11 -or @($cost.pages[0].data).Count -ne 1 -or @($cost.pages[1].data).Count -ne 2) {
    throw "Cost filter expected 11 matching SKUs and complete requested pages"
}

$isolated = Invoke-Ranges $otherParent (New-Query) @(@{ offset = 0; limit = 24 })
if ($isolated.total -ne 0 -or @($isolated.pages[0].data).Count -ne 0) {
    throw "SKU range query crossed the parent product boundary"
}

$invalid = @(
    @(@{ offset = 0; limit = 0 }),
    @(@{ offset = 0; limit = 1001 }),
    @(@{ offset = 0; limit = 1 }, @{ offset = 1; limit = 1 }, @{ offset = 2; limit = 1 }),
    @(@{ offset = 0; limit = 1000 }, @{ offset = 1000; limit = 1001 })
)
$rejected = foreach ($ranges in $invalid) {
    $body = @{ query = (New-Query); ranges = $ranges } | ConvertTo-Json -Depth 10 -Compress
    $response = Invoke-WebRequest -Method Post -Uri "$base/v1/tenants/$tenant/products/$parent/skus/query-ranges" -Headers $headers -ContentType 'application/json' -Body $body -SkipHttpErrorCheck
    if ($response.StatusCode -ne 400) {
        throw "Invalid SKU ranges expected HTTP 400, got $($response.StatusCode)"
    }
    [pscustomobject]@{ ranges = $ranges; status = $response.StatusCode }
}

[pscustomobject]@{
    default = @{ total = $default.total; codes = @('QA-SF-024', 'QA-SF-014', 'QA-SF-013') }
    code_ascending = @{ total = $codeAsc.total; codes = @('QA-SF-001', 'QA-SF-011', 'QA-SF-012') }
    cost_min_2400 = @{ total = $cost.total; page_counts = @(@($cost.pages[0].data).Count, @($cost.pages[1].data).Count) }
    parent_isolation = @{ parent = $otherParent; total = $isolated.total }
    rejected = $rejected
} | ConvertTo-Json -Depth 10
