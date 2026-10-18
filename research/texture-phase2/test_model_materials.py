import unittest
from export_model_materials import tag_evidence, next_context, audit_model, TARGETS


def item(value="", name="UserPropBuffer", is_string=True):
    return {"extra_name": name, "value": value, "is_string": is_string}


class MaterialTests(unittest.TestCase):
    def test_named_presence_ignores_value(self):
        self.assertIn("RepColor", tag_evidence([item("0", "RepColor", False)], ["n"]))
        self.assertNotIn("RepColor", tag_evidence([item("", "repcolor", False)], ["n"]))
        self.assertEqual(tag_evidence([item("Eq", "RepColor")], ["n"]), {})
        self.assertEqual(set(tag_evidence([item("SG", "RepColor")], ["n"])), {"Sg"})
        with self.assertRaisesRegex(ValueError, "extra-array"):
            tag_evidence([item("Eq", "RepColor"), item("Sg")], ["n"])

    def test_case_crlf_and_assignment(self):
        found = tag_evidence([item("unrelated\r\nsg2\r\nrepcolor=0")], ["n"])
        self.assertEqual(set(found), {"Sg2", "RepColor"})
        self.assertEqual(found["Sg2"]["value"], "unrelated\r\nsg2\r\nrepcolor=0")

    def test_no_whitespace_or_lf_guess(self):
        for text in (" Sg", "Sg ", "tag\nSg", "Sg=one=two", "RepColor="):
            self.assertEqual(tag_evidence([item(text)], ["n"]), {}, text)
        self.assertIn("RepColor", tag_evidence([item("RepColor=\r\n")], ["n"]))

    def test_geometry_inherits_but_node_resets(self):
        own = tag_evidence([item("Sg")], ["parent"])
        parent = next_context(False, {}, own)
        self.assertIn("shadow", next_context(True, parent, {}))
        intermediate = next_context(False, parent, {})
        self.assertEqual(next_context(True, intermediate, {}), {})

    def test_multiple_strings_do_not_guess_mutation_order(self):
        for text in ("repcolor=0", "RepColor\r\nSg"):
            with self.assertRaisesRegex(ValueError, "extra-array"):
                tag_evidence([item("Eq"), item(text)], ["n"])
        self.assertEqual(set(tag_evidence([item("repcolor"), item("Sg2")], ["n"])), {"RepColor", "Sg2"})

    def test_shadow_priority_and_recolor_reset(self):
        own = tag_evidence([item("RepColor\r\nSg\r\nSg2")], ["n"])
        context = next_context(False, {}, own)
        self.assertEqual(context["shadow"]["tag"], "Sg2")
        self.assertNotIn("team", context)
        recolor = next_context(False, {}, tag_evidence([item("repcolor")], ["n"]))
        self.assertIn("team", next_context(True, recolor, {}))
        self.assertEqual(next_context(False, recolor, {}), {})

    def test_actual_corps(self):
        record = audit_model(TARGETS[0])
        self.assertEqual(record["mesh_count"], 1)
        self.assertTrue(record["corrections"][0]["team_color_mask"])
        self.assertFalse(record["corrections"][0]["skip"])

    def test_actual_pack(self):
        record = audit_model(TARGETS[1])
        self.assertEqual(record["mesh_count"], 72)
        self.assertEqual(sum(row["skip"] for row in record["corrections"]), 15)
        self.assertEqual(sum(row["team_color_mask"] for row in record["corrections"]), 10)
        self.assertEqual([row["mesh_index"] for row in record["corrections"]], list(range(72)))

    def test_actual_oil_name_is_not_tag(self):
        record = audit_model(TARGETS[2])
        self.assertEqual(record["mesh_count"], 3)
        self.assertFalse(any(row["skip"] for row in record["corrections"]))
        sg01 = next(row for row in record["corrections"] if row["name"] == "Sg01")
        self.assertTrue(sg01["team_color_mask"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
