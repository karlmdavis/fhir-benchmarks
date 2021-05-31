# FHIR Benchmarks: Site

This directory is the root of the project's website,
  which is used to publish benchmark results.

The site can be rendered/built using [Zola](https://www.getzola.org/),
  a static site generator similar to Jekyll:

    $ cd fhir-bench-site
    $ zola build

The site is automatically built from its GitHub project
  and published using [Netlify](https://www.netlify.com/).
It can be viewed here:
  <https://fhir-benchmarks.com/>.