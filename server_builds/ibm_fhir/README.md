# [IBM FHIR Server](https://github.com/IBM/FHIR)

The [IBM FHIR Server](https://github.com/IBM/FHIR)
  is an open source FHIR server.
It is written in Java and supports a number of backend databases:
  Derby, PostgreSQL, and DB2.
Here, we run it with PostgreSQL.

To start the server, use the provided shell script as if it were `docker-compose`:

    $ ./docker_compose_ibm_fhir.sh up --detach

To stop the server, Docker Compose can be used as normal:

    $ ./docker_compose_ibm_fhir down

The setup here is a bit convoluted,
  but we're downloading and running the `demo/` provided by the IBM FHIR repo itself:
<https://github.com/IBM/FHIR/blob/main/demo>.
