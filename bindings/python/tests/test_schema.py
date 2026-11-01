import json

import pytest

import finalytics as fl

from conftest import ROOT

jsonschema = pytest.importorskip("jsonschema")


def test_documents_follow_the_published_schema(portfolio, market):
    schema = json.loads((ROOT / "schema/portfolio-document.schema.json").read_text())
    document = json.loads(fl.write_document(portfolio, market))
    jsonschema.validate(document, schema)
    document["format_version"] = 2
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(document, schema)
