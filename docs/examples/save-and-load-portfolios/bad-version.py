import finalytics as fl

try:
    fl.Portfolio.read(
        '{"format":"finalytics.portfolio","format_version":2,'
        '"portfolio":{}}'
    )
except fl.ValidationError as error:
    for issue in error.issues:
        print(issue.path, "|", issue.message)
