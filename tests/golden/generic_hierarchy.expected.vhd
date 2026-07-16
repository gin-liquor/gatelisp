library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_register is
  generic (
    gl_g0 : positive := 8
  );
  port (
    gl_p0_clk : in std_logic;
    gl_p1_input : in unsigned(gl_g0 - 1 downto 0);
    gl_p2_value : out unsigned(gl_g0 - 1 downto 0)
  );
end entity gl_m0_generic_register;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_register is
  signal gl_s2_value : unsigned(gl_g0 - 1 downto 0);
  signal gl_s3_stored : unsigned(gl_g0 - 1 downto 0) := resize(unsigned'(x"0000000000000000"), gl_g0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_value <= gl_s3_stored;
  end process gl_comb_0;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      gl_s3_stored <= gl_p1_input;
    end if;
  end process gl_seq_0;
  gl_p2_value <= gl_s2_value;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m1_generic_top is
  generic (
    gl_g1 : positive := 16
  );
  port (
    gl_p4_clk : in std_logic;
    gl_p5_input : in unsigned(gl_g1 - 1 downto 0);
    gl_p6_value : out unsigned(gl_g1 - 1 downto 0)
  );
end entity gl_m1_generic_top;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m1_generic_top is
  signal gl_s6_value : unsigned(gl_g1 - 1 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_i0_register0 : entity work.gl_m0_generic_register
    generic map (
      gl_g0 => gl_g1
    )
    port map (
      gl_p0_clk => gl_p4_clk,
      gl_p1_input => gl_p5_input,
      gl_p2_value => gl_s6_value
    );
  gl_p6_value <= gl_s6_value;
end architecture rtl;
